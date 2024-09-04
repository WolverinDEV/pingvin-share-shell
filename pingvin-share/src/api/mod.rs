mod config;
use std::{
    borrow::Cow,
    io::SeekFrom,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;
pub use config::*;

mod share;
use futures::StreamExt;
use rand::{distributions::Alphanumeric, Rng};
use reqwest::{header::HeaderMap, Body, Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
pub use share::*;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
    time,
};
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct PingvinApi {
    base_url: Url,
    http_client: Client,

    authentication_headers: HeaderMap,
}

impl PingvinApi {
    pub fn new(mut base_url: Url) -> anyhow::Result<Self> {
        let _ = base_url.set_password(None);
        let _ = base_url.set_username("");
        Ok(Self {
            base_url,
            http_client: Client::builder().build()?,

            authentication_headers: HeaderMap::new(),
        })
    }

    pub async fn login(&mut self, username: &str, password: &str) -> anyhow::Result<bool> {
        #[derive(Serialize)]
        struct Request<'a> {
            username: &'a str,
            password: &'a str,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            access_token: String,
        }

        let response = self
            .http_client
            .post(self.base_url.join("auth/signIn")?)
            .json(&Request { password, username })
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Ok(false);
        }

        response.error_for_status_ref()?;
        let response = response.json::<Response>().await?;

        self.authentication_headers.insert(
            "Cookie",
            format!("access_token={}", response.access_token).parse()?,
        );
        Ok(true)
    }

    pub async fn public_config(&self) -> anyhow::Result<PublicConfiguration> {
        let response = reqwest::get(self.base_url.join("configs")?)
            .await?
            .error_for_status()?;
        Ok(PublicConfiguration::new(response.json().await?))
    }

    pub fn create_share(&self) -> ShareBuilder {
        ShareBuilder {
            api: self,

            id: None,
            name: None,
            description: None,

            expiration: ExpireDuration::Never,
            files: vec![],
            recipients: vec![],

            security: ShareSecurityOptions::default(),
            event_callback: Box::new(|_| {}),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UploadProgress {
    pub files_total: u64,
    pub files_uploaded: u64,
    pub files_failed: u64,

    pub file_current: PathBuf,
    pub file_length: u64,
    pub file_bytes_uploaded: u64,
}

#[derive(Debug, Clone)]
pub enum UploadEvent {
    ShareCreated {
        share_id: String,
    },
    ShareCompleted,

    UploadProgress(UploadProgress),
    UploadError {
        file: PathBuf,
        error: Arc<anyhow::Error>,
    },
}

pub type UploadEventCallback = dyn Fn(UploadEvent) + 'static;
pub struct ShareBuilder<'a> {
    api: &'a PingvinApi,

    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    expiration: ExpireDuration,
    recipients: Vec<String>,
    security: ShareSecurityOptions,

    files: Vec<PathBuf>,
    event_callback: Box<UploadEventCallback>,
}

impl<'a> ShareBuilder<'a> {
    pub fn set_id(&mut self, id: String) -> &mut Self {
        self.id = Some(id);
        self
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    pub fn set_description(&mut self, description: String) -> &mut Self {
        self.description = Some(description);
        self
    }

    #[allow(unused)]
    pub fn set_expiration(&mut self, expiration: ExpireDuration) -> &mut Self {
        self.expiration = expiration;
        self
    }

    #[allow(unused)]
    pub fn set_security_options(&mut self, security: ShareSecurityOptions) -> &mut Self {
        self.security = security;
        self
    }

    pub fn add_file(&mut self, file: PathBuf) -> &mut Self {
        self.files.push(file);
        self
    }

    pub fn with_callback(&mut self, callback: impl Fn(UploadEvent) + 'static) -> &mut Self {
        self.event_callback = Box::new(callback);
        self
    }

    /* TODO: Some kind of process monitor */
    pub async fn upload(self) -> anyhow::Result<String> {
        let share_config = self.api.public_config().await?;

        let chunk_size = share_config
            .get_number("share.chunkSize")
            .context("missing chunk size config value")?
            .as_u64()
            .context("chunk size should be an u64")? as usize;

        log::debug!("Uploading files using a chunk size of {} bytes", chunk_size);

        let share_id = self.create_share().await?;
        (*self.event_callback)(UploadEvent::ShareCreated {
            share_id: share_id.clone(),
        });

        let mut progress = UploadProgress::default();
        progress.files_total = self.files.len() as u64;
        progress.file_length = self
            .files
            .iter()
            .filter_map(|file| file.metadata().ok())
            .map(|meta| meta.len())
            .sum();

        for file in self.files.iter() {
            progress.file_current = file.clone();
            (*self.event_callback)(UploadEvent::UploadProgress(progress.clone()));

            let result = self
                .upload_file(&share_id, file, chunk_size, &mut progress)
                .await;

            match result {
                Ok(_file_id) => {
                    progress.files_uploaded += 1;
                }
                Err(err) => {
                    progress.files_failed += 1;

                    log::error!("Failed to upload {}: {}", file.display(), err);
                    (*self.event_callback)(UploadEvent::UploadError {
                        file: file.clone(),
                        error: Arc::new(err),
                    });
                }
            }
            (*self.event_callback)(UploadEvent::UploadProgress(progress.clone()));
        }

        if let Err(err) = self.complete_share(&share_id).await {
            log::warn!("Failed to mark share {} as completed: {}", share_id, err);
        }
        (*self.event_callback)(UploadEvent::ShareCompleted);
        Ok(share_id)
    }

    async fn create_share(&self) -> anyhow::Result<String> {
        #[derive(Serialize)]
        struct Request<'a> {
            id: Cow<'a, str>,
            expiration: &'a ExpireDuration,

            #[serde(skip_serializing_if = "Option::is_none")]
            name: &'a Option<String>,

            #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(default)]
            description: &'a Option<String>,

            recipients: &'a Vec<String>,

            security: &'a ShareSecurityOptions,
        }

        #[derive(Deserialize)]
        struct Response {
            id: String,
            /* other members have been omitted */
        }

        let response = self
            .api
            .http_client
            .post(self.api.base_url.join("shares")?)
            .json(&Request {
                id: self.id.as_ref().map_or_else(
                    || {
                        rand::thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(7)
                            .map(char::from)
                            .collect::<String>()
                            .into()
                    },
                    Cow::from,
                ),
                name: &self.name,
                description: &self.description,
                expiration: &self.expiration,
                recipients: &self.recipients,
                security: &self.security,
            })
            .headers(self.api.authentication_headers.clone())
            .send()
            .await?
            .error_for_status()?;

        let response: Response = response.json().await?;
        Ok(response.id)
    }

    async fn upload_file(
        &self,
        share_id: &str,
        file_path: &Path,
        chunk_size: usize,
        progress: &mut UploadProgress,
    ) -> anyhow::Result<String> {
        #[derive(Default, Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Response {
            pub id: String,
        }

        let url = self
            .api
            .base_url
            .join(&format!("shares/{}/files", share_id))?;

        let file = File::options()
            .read(true)
            .open(&file_path)
            .await
            .context("open")?;

        let file_length = file.metadata().await?.len() as usize;
        let file_name = file_path
            .file_name()
            .context("expected a file name")?
            .to_string_lossy();

        let chunk_count = {
            let mut chunks = (file_length / chunk_size).min(1);
            if chunks * chunk_size < file_length {
                chunks += 1;
            }
            chunks
        };
        let mut current_chunk_index = 0;

        let bytes_uploaded = Arc::new(AtomicU64::new(0));
        progress.file_length = file_length as u64;

        let mut file_id: Option<String> = None;
        while current_chunk_index < chunk_count {
            let debug_info = format!(
                "chunk {}/{} ({}/{} bytes)",
                current_chunk_index,
                chunk_count,
                current_chunk_index * chunk_size,
                file_length
            );

            let current_chunk_length = {
                let chunk_base = current_chunk_index * chunk_size;
                if chunk_base > file_length {
                    0
                } else {
                    (file_length - chunk_base).min(chunk_size)
                }
            };

            let file_chunk = {
                let mut chunk = file.try_clone().await?;
                chunk
                    .seek(SeekFrom::Start((current_chunk_index * chunk_size) as u64))
                    .await
                    .with_context(|| format!("seek to chunk {}", debug_info))?;
                chunk.take(chunk_size as u64)
            };

            let mut query: Vec<(&str, Cow<'_, str>)> = Vec::with_capacity(4);
            if let Some(id) = &file_id {
                query.push(("id", id.into()));
            }
            query.push(("name", file_name.clone()));
            query.push(("chunkIndex", format!("{}", current_chunk_index).into()));
            query.push(("totalChunks", format!("{}", chunk_count).into()));

            let body_stream = FramedRead::new(file_chunk, BytesCodec::new());
            let body_stream = body_stream.map({
                let bytes_uploaded = bytes_uploaded.clone();
                move |chunk| {
                    if let Ok(chunk) = &chunk {
                        bytes_uploaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                    }

                    chunk
                }
            });

            log::debug!("Uploading {}", debug_info);
            let mut upload = self
                .api
                .http_client
                .post(url.clone())
                .body(Body::wrap_stream(body_stream))
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", format!("{}", current_chunk_length))
                .query(&query)
                .headers(self.api.authentication_headers.clone())
                .send();

            let mut stats_poll = time::interval(Duration::from_millis(500));
            let response = loop {
                tokio::select! {
                    _ = stats_poll.tick() => {
                        progress.file_bytes_uploaded = bytes_uploaded.load(Ordering::Relaxed);
                        (*self.event_callback)(UploadEvent::UploadProgress(progress.clone()));
                    },
                    result = &mut upload => break result?,
                }
            };
            let response = response.error_for_status()?.json::<Response>().await?;

            file_id = Some(response.id);
            current_chunk_index += 1;

            progress.file_bytes_uploaded = bytes_uploaded.load(Ordering::Relaxed);
            (*self.event_callback)(UploadEvent::UploadProgress(progress.clone()));
        }

        Ok(file_id.context("failed to obtain a file id")?)
    }

    async fn complete_share(&self, share_id: &str) -> anyhow::Result<()> {
        #[derive(Default, Debug, Serialize)]
        struct Payload<'a> {
            id: &'a str,
        }

        let url = self
            .api
            .base_url
            .join(&format!("shares/{}/complete", share_id))?;

        let _response = self
            .api
            .http_client
            .post(url)
            .json(&Payload { id: share_id })
            .headers(self.api.authentication_headers.clone())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
