# Versões e arquiteturas

O workflow `.github/workflows/release.yml` é acionado por tags SemVer estritas,
como `v0.1.0`. A tag precisa corresponder ao campo `version` do `Cargo.toml`.

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
4. Crie e envie a tag correspondente:

```bash
git tag -a v0.2.0 -m "AgendarX v0.2.0"
git push origin v0.2.0
```

O GitHub Actions cria a Release com notas automáticas, pacotes e arquivos `.sha256`.
Também publica no GHCR as tags `0.2.0`, `0.2`, `0` e `latest`.

## Usando a imagem

Substitua proprietário e repositório pelo caminho exibido em **Packages**:

```bash
docker pull ghcr.io/proprietario/agendarx:0.2.0
docker run --rm -p 12000:12000 ghcr.io/proprietario/agendarx:0.2.0
```

O Docker seleciona automaticamente AMD64, ARM64 ou ARMv7 a partir do manifesto.

## Usando um pacote binário

```bash
sha256sum -c agendarx-0.2.0-linux-riscv64.tar.gz.sha256
tar -xzf agendarx-0.2.0-linux-riscv64.tar.gz
cd agendarx-0.2.0-linux-riscv64
cp .env.example .env
./agendarx
```

O sistema precisa fornecer glibc compatível e certificados CA. Edite `.env` antes
da primeira inicialização e mantenha o diretório `data/` em armazenamento persistente.
