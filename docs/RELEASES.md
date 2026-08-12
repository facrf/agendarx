# Versões e arquiteturas

O workflow `.github/workflows/release.yml` é acionado por tags SemVer estritas,
como `v0.1.0`. A tag precisa corresponder ao campo `version` do `Cargo.toml`.
As versões publicadas ficam em
[github.com/facrf/agendarx/releases](https://github.com/facrf/agendarx/releases).
O andamento de cada publicação aparece no
[workflow Publicar versão](https://github.com/facrf/agendarx/actions/workflows/release.yml).

## Execução exclusiva no GitHub

As pastas `.gitea/workflows/` e `.forgejo/workflows/` existem sem arquivos YAML
para impedir que Gitea e Forgejo usem `.github/workflows` como fallback. Os jobs
também validam `github.server_url == 'https://github.com'`, protegendo instalações
que tenham personalizado os diretórios de workflow.

Para remover também a aba e o histórico de Actions, desative a unidade no servidor:

- Gitea: **Settings > Enable Repository Actions** (desmarcar);
- Forgejo: **Settings > Units > Overview > Actions** (desmarcar).

Não use `[skip ci]` nos commits enviados ao Gitea neste projeto: a mesma mensagem é
espelhada no GitHub e também impediria a CI desejada no destino.

## Artefatos gerados

| Plataforma | Target Rust | Pacote no GitHub Release | Imagem GHCR |
|---|---|---:|---:|
| Intel/AMD 64 bits | `x86_64-unknown-linux-gnu` | Sim | Sim (`linux/amd64`) |
| ARM 64 bits | `aarch64-unknown-linux-gnu` | Sim | Sim (`linux/arm64`) |
| ARM 32 bits v7 | `armv7-unknown-linux-gnueabihf` | Sim | Sim (`linux/arm/v7`) |
| RISC-V 64 bits | `riscv64gc-unknown-linux-gnu` | Sim | Não |

O manifesto Docker cobre as arquiteturas disponíveis em todas as imagens-base
oficiais utilizadas. Como essas bases ainda não publicam RISC-V, essa arquitetura é
entregue como pacote executável do GitHub Release. Cada pacote contém o binário, o
frontend compilado, `.env.example`, README e o exemplo do Portainer.

## Criando uma versão

1. Atualize `version` no `Cargo.toml`.
2. Registre as mudanças no `CHANGELOG.md`.
3. Faça merge e confirme que o workflow `CI` passou.
4. Envie o commit ao Gitea e aguarde o espelho atualizar `main` no GitHub.
5. Crie a tag anotada no mesmo commit e envie-a ao Gitea:

```bash
git remote get-url origin
# neste projeto, origin pode apontar para o Gitea que espelha no GitHub
git push origin main
git tag -a v0.2.1 -m "AgendarX v0.2.1"
git push origin v0.2.1
```

Confirme que a tag também apareceu em `github.com/facrf/agendarx`; somente então o
workflow é disparado. O GitHub Actions cria a Release com notas automáticas,
pacotes e arquivos `.sha256`. Também publica no GHCR as tags `0.2.1`, `0.2`, `0`
e `latest`.

Na primeira publicação, o GitHub pode criar o pacote GHCR como privado. Para
permitir `docker pull` sem login, abra **Packages > agendarx > Package settings >
Change visibility**, escolha **Public** e confirme. Essa mudança é permanente.

## Usando a imagem

Use a imagem publicada pelo projeto no GitHub Container Registry:

```bash
docker pull ghcr.io/facrf/agendarx:0.2.1
docker run --rm -p 12000:12000 ghcr.io/facrf/agendarx:0.2.1
```

O Docker seleciona automaticamente AMD64, ARM64 ou ARMv7 a partir do manifesto.

## Usando um pacote binário

```bash
sha256sum -c agendarx-0.2.1-linux-riscv64.tar.gz.sha256
tar -xzf agendarx-0.2.1-linux-riscv64.tar.gz
cd agendarx-0.2.1-linux-riscv64
cp .env.example .env
./agendarx
```

O sistema precisa fornecer glibc compatível e certificados CA. Edite `.env` antes
da primeira inicialização e mantenha o diretório `data/` em armazenamento persistente.
