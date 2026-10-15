# Plano — spec 018

- `hmd-app::ai`: `KeyVault` (trait), `MemoryVault`, `ai_status/ai_connect/ai_disconnect/ai_classify`.
- `hmd-store::record_ai_call` (auditoria só com códigos).
- `hmd-ipc`: cofre e transporte injetáveis; DTOs `AiStatusDto`, `AiClassificationDto`.
- Shell: `SystemVault` (`keyring =4.2.0`), `open_ai_console` (`tauri-plugin-opener =2.7.0`, URL fixa),
  `ai_connect_from_clipboard` (`tauri-plugin-clipboard-manager =2.4.0`). Nenhuma janela recebe as
  permissões próprias dos plugins. ADR-028.
- UI: componente `app-ai-connect` (boas-vindas e Configurações) e texto livre na tela inicial.
