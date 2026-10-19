# Plano — spec 021

`hmd_ai::classify_with_findings` (`request_body` com `verification_codes` + sanitizador de
códigos); `hmd_app`: `ai_prepare_with_findings`/`ai_classify_with_findings` (gurda códigos no
`AiCall`); IPC `ai_classify(text, findings)`; shell Tauri passa `findings` opcional; UI: o
relato vai para a `Session` (Home), o cartão de sugestão na `DiagnosisPage` quando
`isFallback` (1 chamada por atendimento). ADR-033.
