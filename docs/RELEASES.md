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

1. Atualize a versão em `Cargo.toml`, `Cargo.lock`, `frontend/package.json` e
   `frontend/package-lock.json`, além das referências de imagem no README, na
   documentação e nos exemplos de implantação.
2. Registre as mudanças no `CHANGELOG.md`.
3. Faça merge e confirme que o workflow `CI` passou.
4. Confirme que o remoto `github` aponta para `https://github.com/facrf/agendarx.git`.
5. Envie o commit e a tag anotada diretamente para o GitHub:

```bash
git remote get-url github
# https://github.com/facrf/agendarx.git
git push github main
git tag -a v0.6.5 -m "AgendarX v0.6.5"
git push github v0.6.5
```

Não use `git push origin` nesse fluxo: neste projeto `origin` é o espelho Gitea.
O envio direto ao remoto `github` evita publicar a mudança no Gitea.

A tag dispara o workflow no GitHub. Acompanhe-o em **Actions > Publicar versão**.
Só considere a imagem pronta quando o job **Imagem Docker multi-arquitetura**
terminar com sucesso e a tag aparecer em
[Packages > agendarx](https://github.com/facrf/agendarx/pkgs/container/agendarx).
O workflow cria a Release com notas automáticas, pacotes e arquivos `.sha256` e
publica no GHCR as tags `0.6.5`, `0.6`, `0` e `latest`. O envio da tag não espera
o build: o GitHub continua a compilação e você pode fechar o terminal.

Na primeira publicação, o GitHub pode criar o pacote GHCR como privado. Para
permitir `docker pull` sem login, abra **Packages > agendarx > Package settings >
Change visibility**, escolha **Public** e confirme. Essa mudança é permanente.

## Usando a imagem

Use a imagem publicada pelo projeto no GitHub Container Registry:

```bash
docker pull ghcr.io/facrf/agendarx:0.6.5
docker run --rm -p 12000:12000 ghcr.io/facrf/agendarx:0.6.5
```

O Docker seleciona automaticamente AMD64, ARM64 ou ARMv7 a partir do manifesto.

## Usando um pacote binário

```bash
sha256sum -c agendarx-0.6.5-linux-riscv64.tar.gz.sha256
tar -xzf agendarx-0.6.5-linux-riscv64.tar.gz
cd agendarx-0.6.5-linux-riscv64
cp .env.example .env
./agendarx
```

O sistema precisa fornecer glibc compatível e certificados CA. Edite `.env` antes
da primeira inicialização e mantenha o diretório `data/` em armazenamento persistente.
