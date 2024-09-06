!include "x64.nsh"
!include "MUI2.nsh"
!include "InstallOptions.nsh"

; Opt for the best compression
SetCompress force
SetCompressor lzma
; SetCompress off

##!define PINGVIN_EXE_PATH                  ## Path of the pingvin executable to use
##!define PINGVIN_SHELL_EXTENSION_PATH      ## Path of the pingvin shell extension dll

!define PRODUCT_NAME "Pingvin Shell Extension"
!define PRODUCT_APP_ID "dev.wolveringer.pingvin-share-shell"

; Check for all required variables
!ifndef PINGVIN_EXE_PATH
  !error "PINGVIN_EXE_PATH must be specified"
!endif

!ifndef PINGVIN_SHELL_EXTENSION_PATH
  !error "PINGVIN_SHELL_EXTENSION_PATH must be specified"
!endif

!define INSTALLER_FILE_NAME "Pingvin Shell Setup.exe"
Name "${PRODUCT_NAME}"
OutFile "${INSTALLER_FILE_NAME}"
InstallDir "$PROGRAMFILES\${PRODUCT_NAME}" ; Default install dir
InstallDirRegKey HKLM "Software\${PRODUCT_NAME}" "InstallDir" ; load the install dir from registry
CRCCheck on
InstProgressFlags smooth
WindowIcon on
ShowInstDetails show
; RequestExecutionLevel Admin

!define PRODUCT_PUBLISHER "Markus Hadenfeldt"
!define /date PRODUCT_COPYRIGHT " %Y ${PRODUCT_PUBLISHER}"
!define /date PRODUCT_SEM_VERSION "%Y.%m.%d.0"
!define /date PRODUCT_VERSION "%Y.%m.%d"

VIProductVersion "${PRODUCT_SEM_VERSION}"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "CompanyName" "${PRODUCT_PUBLISHER}"
VIAddVersionKey "LegalCopyright" "${PRODUCT_COPYRIGHT}"
VIAddVersionKey "FileDescription" "Pingvin Shell Extension"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
BrandingText "${PRODUCT_PUBLISHER}"

; MUI2 Setup
!define MUI_ICON "resources\icon.ico"
; !define MUI_UNICON "resources\icon_uninstall.ico"
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_ABORTWARNING

; The installer
!include "PingvinConfig.nsdinc"
!insertmacro MUI_PAGE_WELCOME
Page custom fnc_PingvinConfig_Show PingvinConfig_OnNext
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; The uninstaller
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_COMPONENTS
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

!addplugindir "plugins"

Var ConfigServerUrlFinal
Var ConfigServerUrl
Var ConfigUsername
Var ConfigPassword
Var ConfigCustomDisplayName
Var ConfigCustomIcon

Section "Shell Extension" SectionShellExtension
    ; Mark section as required
    SectionIn 1 RO

    SetOverwrite ifnewer

    ; Install files
    SetOutPath "$INSTDIR\"
    File "resources\icon.ico"
    File "resources\icon.png"
    File /oname=pingvin-share.exe "${PINGVIN_EXE_PATH}"
    File /oname=pingvin_share_shell.dll "${PINGVIN_SHELL_EXTENSION_PATH}"

    ; Create configuration
    WriteINIStr $INSTDIR\config_shell.ini "default" "pingvin-args" "-s,$ConfigServerUrlFinal"
    WriteINIStr $INSTDIR\config_shell.ini "default" "pingvin-exe" "$INSTDIR\pingvin-share.exe"

    ${IF} $ConfigCustomDisplayName != ""
        WriteINIStr $INSTDIR\config_shell.ini "default" "menu-title" "$ConfigCustomDisplayName"
    ${ENDIF}

    ${IF} $ConfigCustomIcon != ""
        WriteINIStr $INSTDIR\config_shell.ini "default" "menu-icon" "$ConfigCustomIcon"
    ${ENDIF}

    ExecWait '$SYSDIR\regsvr32.exe /s "$INSTDIR\pingvin_share_shell.dll"'

    WriteRegStr HKLM "Software\Classes\AppUserModelId\${PRODUCT_APP_ID}" "DisplayName" "Pingvin Share"
    WriteRegStr HKLM "Software\Classes\AppUserModelId\${PRODUCT_APP_ID}" "IconUri" "$INSTDIR\icon.png"
    WriteRegStr HKLM "Software\Classes\AppUserModelId\${PRODUCT_APP_ID}" "IconBackgroundColor" "FFDDDDDD"
    
    CreateShortcut "$SMPROGRAMS\${PRODUCT_NAME}.lnk" "$INSTDIR\pingvin-share.exe" "" "$INSTDIR\icon.png"
    ApplicationID::Set "$SMPROGRAMS\${PRODUCT_NAME}.lnk" "${PRODUCT_APP_ID}"
    Pop $0
    ${IF} $0 = -1
        messagebox MB_OK "Failed to set app id for Pingvin shortcut"
    ${ENDIF}
SectionEnd

Section -FinishSection
    WriteRegStr HKLM "Software\${PRODUCT_NAME}" "InstallDir" "$INSTDIR"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd


Section "un.Pingvin Shell Extension" Uninstall
    ; Mark section as required
    SectionIn 1 RO

    DeleteRegKey HKLM "SOFTWARE\${PRODUCT_NAME}\InstallDir"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
    DeleteRegKey HKLM "Software\Classes\AppUserModelId\${PRODUCT_APP_ID}"

    ExecWait '$SYSDIR\regsvr32.exe /s "$INSTDIR\pingvin_share_shell.dll" /u'

    Delete "$SMPROGRAMS\${PRODUCT_NAME}.lnk"
    Delete "$INSTDIR\Uninstall.exe"
    Delete "$INSTDIR\pingvin-share.exe"
    Delete "$INSTDIR\pingvin_share_shell.dll"
    Delete "$INSTDIR\icon.ico"
    Delete "$INSTDIR\icon.png"
SectionEnd

Section /o "un.Config files" UninstallUserDataConfigs
    Delete "$INSTDIR\config_shell.ini"
    Delete "$INSTDIR\log4rs-shell.yml"
    Delete "$INSTDIR\log4rs.yml"
SectionEnd

Section -UninstallDeleteRootDir
    ; delete install dir if it's completely empty
    RMDir "$INSTDIR"
SectionEnd

; Modern install component descriptions
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SectionShellExtension} "Install the ${PRODUCT_NAME}"

    !insertmacro MUI_DESCRIPTION_TEXT ${Uninstall} "Delete all ${PRODUCT_NAME} files"
    !insertmacro MUI_DESCRIPTION_TEXT ${UninstallUserDataConfigs} "Delete all user editable config files"
!insertmacro MUI_FUNCTION_DESCRIPTION_END



; Pingvin config form handlers 
Var CheckboxState

Function PingvinConfig_OnCreate
    ${NSD_SetText} $hCtl_PingvinConfig_TextServerUrl "$ConfigServerUrl"

    ${IF} $ConfigUsername != "" 
    ${ORIF} $ConfigUsername != ""
        EnableWindow $hCtl_PingvinConfig_TextPassword 1
        EnableWindow $hCtl_PingvinConfig_TextUsername 1
        ${NSD_SetState} $hCtl_PingvinConfig_CheckboxAuth ${BST_CHECKED}
    ${ELSE}
        EnableWindow $hCtl_PingvinConfig_TextPassword 0
        EnableWindow $hCtl_PingvinConfig_TextUsername 0
    ${ENDIF} 
    ${NSD_SetText} $hCtl_PingvinConfig_TextUsername "$ConfigUsername"
    ${NSD_SetText} $hCtl_PingvinConfig_TextPassword "$ConfigPassword"

    ${IF} $ConfigCustomDisplayName != ""
        EnableWindow $hCtl_PingvinConfig_TextCustomDisplayName 1
        ${NSD_SetState} $hCtl_PingvinConfig_CheckCustomDisplayName ${BST_CHECKED}
    ${ELSE}
        EnableWindow $hCtl_PingvinConfig_TextCustomDisplayName 0
    ${ENDIF}
    ${NSD_SetText} $hCtl_PingvinConfig_TextCustomDisplayName "$ConfigCustomDisplayName"

    ${IF} $ConfigCustomIcon != ""
        EnableWindow $hCtl_PingvinConfig_TextCustomIcon 1
        ${NSD_SetState} $hCtl_PingvinConfig_CheckCustomIcon ${BST_CHECKED}
    ${ELSE}
        EnableWindow $hCtl_PingvinConfig_TextCustomIcon 0
    ${ENDIF}
    ${NSD_SetText} $hCtl_PingvinConfig_TextCustomIcon "$ConfigCustomIcon"
FunctionEnd

Function CheckboxAuth_OnClick
    ${NSD_GetState} $hCtl_PingvinConfig_CheckboxAuth $CheckboxState
    ${If} $CheckboxState == ${BST_UNCHECKED}
        EnableWindow $hCtl_PingvinConfig_TextPassword 0
        EnableWindow $hCtl_PingvinConfig_TextUsername 0
    ${Else}
        EnableWindow $hCtl_PingvinConfig_TextPassword 1
        EnableWindow $hCtl_PingvinConfig_TextUsername 1
    ${EndIf}
FunctionEnd

Function CheckCustomDisplayName_OnClick
    ${NSD_GetState} $hCtl_PingvinConfig_CheckCustomDisplayName $CheckboxState
    ${If} $CheckboxState == ${BST_UNCHECKED}
        EnableWindow $hCtl_PingvinConfig_TextCustomDisplayName 0
    ${Else}
        EnableWindow $hCtl_PingvinConfig_TextCustomDisplayName 1
    ${EndIf}
FunctionEnd

Function CheckCustomIcon_OnClick
    ${NSD_GetState} $hCtl_PingvinConfig_CheckCustomIcon $CheckboxState
    ${If} $CheckboxState == ${BST_UNCHECKED}
        EnableWindow $hCtl_PingvinConfig_TextCustomIcon 0
    ${Else}
        EnableWindow $hCtl_PingvinConfig_TextCustomIcon 1
    ${EndIf}
FunctionEnd

Function PingvinConfig_OnNext
    ${NSD_GetText} $hCtl_PingvinConfig_TextServerUrl $ConfigServerUrl
    ${NSD_GetText} $hCtl_PingvinConfig_TextUsername $ConfigUsername
    ${NSD_GetText} $hCtl_PingvinConfig_TextPassword $ConfigPassword
    ${NSD_GetText} $hCtl_PingvinConfig_TextCustomDisplayName $ConfigCustomDisplayName
    ${NSD_GetText} $hCtl_PingvinConfig_TextCustomIcon $ConfigCustomIcon

    nsis_pingvin::BuildServerUrl "$ConfigServerUrl" "$ConfigUsername" "$ConfigPassword"
    Pop $1
    ${IF} $1 == 1
        Pop $ConfigServerUrlFinal
    ${ELSE}
        Pop $2
        messagebox MB_OK "Invalid server configuration:$\r$\n$2"
        Abort
    ${ENDIF}

    nsis_pingvin::ValidateServerUrl "$ConfigServerUrlFinal"
    Pop $1
    ${IF} $1 == 1
        Pop $2
        Pop $3
        messagebox MB_OK "Server configuration valid:$\r$\nName: $2$\r$\nPublic URL: $3"
    ${ELSE}
        Pop $2
        messagebox MB_OK "Valid to validate server URL:$\r$\n$2"
        Abort
    ${ENDIF}
FunctionEnd