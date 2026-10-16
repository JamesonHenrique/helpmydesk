# Como lançar uma versão (F5-REL)

O workflow `.github/workflows/release.yml` roda quando uma tag `vX.Y.Z` chega ao GitHub. Ele confere
que a tag bate com a versão do app e dos dois workspaces, gera os instaladores pelo mesmo workflow
do CI (`installers.yml`, com o helper dentro do NSIS) e abre um **rascunho** de pré-lançamento com
os instaladores e o `SHA256SUMS`. Nada é publicado sem o mantenedor.

## Passo a passo (mantenedor)

1. Num PR, mude a versão nos três lugares: `apps/desktop/src-tauri/tauri.conf.json`,
   `Cargo.toml` (`[workspace.package]`) e `apps/desktop/src-tauri/Cargo.toml`. Os `Cargo.lock`
   mudam junto (`cargo update -w`). Merge com o CI verde.
2. Na `main` atualizada: `git tag -s vX.Y.Z -m "HelpMyDesk vX.Y.Z"` (ou `-a` sem chave GPG) e
   `git push origin vX.Y.Z`.
3. Espere o workflow "Release" terminar e abra o rascunho em Releases.
4. Baixe o `SHA256SUMS` do rascunho e assine **na sua máquina**:
   `gpg --armor --detach-sign SHA256SUMS`. Suba o `SHA256SUMS.asc` no rascunho.
5. Revise as notas (vêm de `docs/release/notas-beta.md`) e publique. Fica marcado como
   pré-lançamento (canal beta).

## Antes do primeiro lançamento

- Chave GPG de lançamento: gere com `gpg --full-generate-key`, guarde a privada fora do repositório
  com cópia offline (como a do kill switch) e publique a pública no README.
- O instalador do Windows ainda não é assinado (S-07): as notas explicam o aviso do SmartScreen.
- A release `kill-switch` usa uma tag fixa; um lançamento novo não mexe nela.
