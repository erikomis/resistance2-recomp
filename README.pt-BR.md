# Resistance 2 — recompilação estática para PC (macOS)

🌐 [English](README.md) · **Português (BR)**

Port nativo do **Resistance 2** (PS3, disco `BCUS98120`) para
**macOS/arm64**, feito com recompilação estática: o código PowerPC e SPU do
jogo é traduzido para C/C++ e compilado como um executável nativo, com
gráficos em Metal. Não é um emulador.

**Ele inicia e chega à primeira fase.** O jogo roda do boot e do aviso de
auto-save, pelos vídeos de abertura e pelos menus, até uma campanha nova, e a
primeira fase é desenhada pela engine. Os vídeos Bink rodam nos seus 30 fps
agora que o decodificador Bink do raw SPU é traduzido em vez de
interpretado. Dentro da fase ainda está lento demais para jogar (uns 5 fps)
e a imagem tem uma borda branca; veja [Estado](#estado).

Construído sobre o [ps3recomp](https://github.com/sp00nznet/ps3recomp), com o
toolkit, a organização e as ferramentas do
[port do Resistance: Fall of Man](https://github.com/erikomis/resistance-recomp)
do mesmo autor. Veja [CREDITS.pt-BR.md](CREDITS.pt-BR.md).

> [!WARNING]
> **Trabalho em andamento, no começo.** Ainda não dá para jogar: a campanha
> começa, mas a primeira fase roda a uns 5 fps. Não há releases: você mesmo
> compila o jogo a partir do seu disco. Só macOS com Apple silicon, com
> controle. O que falta está no [ROADMAP.pt-BR.md](ROADMAP.pt-BR.md).

> **Este repositório não contém dados do jogo, firmware, chaves da Sony nem
> nada traduzido a partir deles.** Cada um gera tudo isso localmente a partir
> da própria cópia legítima. Projeto de fã, sem afiliação com a Sony
> Interactive Entertainment ou a Insomniac Games.

## Estado

Em 29/10/2026:

- **Boot e menus:** Metal, áudio e controle sobem; o aviso de auto-save, os
  vídeos de abertura e os menus rodam até **uma campanha nova**.
- **Controle na primeira tela:** o despachante de entrada do front end lia o
  port 0 como desabilitado antes de existir um jogador local, então o X não
  fazia nada no aviso de auto-save. Um hook bem restrito no despachante
  traduzido (`tools/apply_r2_frontend_patch.py`, aplicado na configuração do
  CMake; chave: `RES_FRONTEND_PORT0=0`) mantém os eventos do port 0
  chegando. Ele não injeta nada.
- **Vídeos a 30 fps:** o decodificador Bink roda num raw SPU e é carregado
  como bytes crus (não um ELF), então o scanner de imagens nunca o via e ele
  rodava no interpretador de SPU a ~3 fps. Agora ele é recortado do EBOOT e
  traduzido (`tools/prepare_r2_bink.py`), junto com o carregador de 320 bytes
  do raw SPU (`tools/prepare_r2_rawspu.py`).
- **A fila de jobs anda:** o módulo de política SPU Wws_Job também é bytes
  crus no EBOOT; ele é traduzido (`tools/prepare_r2_job_pm.py`) e registrado
  pela assinatura. Um despertar perdido no `WAIT_SIGNAL` das tarefas SPURS
  (um job empurrado com a tarefa acordada nunca era visto) travava o
  carregamento da fase; o toolkit agora considera satisfeita a espera numa
  fila lock-free que já tem entradas.
- **New Game:** o jogo consulta a sua "check thread" de save até o LV2 dizer
  que ela acabou (`ESRCH`); o runtime a dava como viva para sempre e o
  "New Game" travava. Corrigido em `sys_ppu_thread_get_priority`.
- **Primeira fase na engine:** a campanha carrega e desenha a primeira fase.

Problemas conhecidos, em andamento:

- **Uns 5 fps na fase.** Os módulos de job WWS do SPU (29 módulos
  `0xC0DEC0DE` no EBOOT) ainda rodam no interpretador de SPU. Traduzi-los
  (`tools/regen_r2_jobmods.sh`) está em andamento: alguns relatam instruções
  não suportadas e ainda precisam ser conferidos um a um.
- **Viewport:** uma borda branca em volta da imagem dentro da fase.
- **Problemas intermitentes** (travadas ocasionais, o encerramento que
  precisa de um kill) em investigação.
- Saves, áudio durante o jogo e qualquer coisa depois da primeira fase não
  foram testados.

## Como funciona

O mesmo caminho do port do Resistance: Fall of Man:

1. **Descriptografar** o `EBOOT.BIN` do seu disco com o RPCS3 em
   `game/BCUS98120/EBOOT.elf`.
2. **Analisar e traduzir** com o `ppu_loader.py` e o `ppu_lifter.py` do
   toolkit: umas 41.500 funções PowerPC viram C++ em `recompiled/BCUS98120/`
   (24 unidades de tradução).
3. **Código SPU:** os ELFs de SPU embutidos no EBOOT são achados e traduzidos
   pelo build (`PORT_SPU_INPUTS`). O Resistance 2 também guarda código SPU
   como bytes crus, que nenhum scanner acha: o módulo de política Wws_Job (VA
   do EBOOT `0x00CF9680`, LS `0xA00`), o decodificador Bink do raw SPU
   (`0x00EB6580`, LS `0x200`), o carregador `igInitRawSpu` (`0x00ED2330`,
   LS `0x10`) e os módulos de job WWS. Os scripts `tools/prepare_r2_*.py` e o
   `tools/regen_r2_jobmods.sh` recortam cada um do seu EBOOT, conferem a
   assinatura com o executável testado e o traduzem.
4. **Compilar** com CMake/Ninja contra o toolkit com o patch: o runtime
   (syscalls do LV2, bibliotecas HLE do firmware, SPURS, o backend RSX em
   Metal, FFmpeg para vídeo) mais o código traduzido, em
   `build/BCUS98120/ResistanceRecomp`, e o pequeno programa `Resistance`, dono
   da janela.

O toolkit é o ps3recomp v0.17.0 (`a6d76c8`) do upstream mais um patch,
[`patches/ps3recomp-a6d76c8-resistance2.patch`](patches/): o patch inteiro do
toolkit do Resistance: Fall of Man mais as mudanças do Resistance 2 (a
correção do despertar perdido no SPURS, `ESRCH` para threads que acabaram,
notificações reais de `sys_lwcond` opcionais com `PS3_LWCOND_EVENTS=1`, um
timebase ancorado na primeira leitura e o log de controle com
`PS3_PAD_LOG=1`).

## Compilando

### O que você precisa

- Um Mac com Apple silicon (macOS com Metal).
- O seu próprio **disco do Resistance 2, `BCUS98120`**, copiado para uma
  imagem e montado, ou extraído em `game/BCUS98120/` (a pasta que tem o
  `PS3_GAME/`).
- O [RPCS3](https://rpcs3.net) para descriptografar o `EBOOT.BIN` (se falhar,
  instale antes o firmware do PS3 no RPCS3). Um ELF descriptografado de outro
  jeito também serve: coloque-o em `game/BCUS98120/EBOOT.elf`.
- Xcode Command Line Tools e Homebrew:

```sh
brew install cmake ninja sdl2 glslang spirv-cross spirv-tools ffmpeg python@3.12
```

### Compilando

```sh
git clone https://github.com/erikomis/resistance2-recomp.git
cd resistance2-recomp
tools/rebuild.sh            # toolkit, decrypt, analyse, lift, spu, build
```

O `tools/rebuild.sh` roda estes passos (rode qualquer um sozinho pelo nome,
por exemplo `tools/rebuild.sh spu build`):

| Passo | O que faz |
| --- | --- |
| `toolkit` | `tools/setup_toolkit.sh`: clona o ps3recomp em `./ps3recomp`, faz checkout do `a6d76c8` e aplica o patch |
| `decrypt` | copia o `EBOOT.BIN` e o `PARAM.SFO` do disco para `game/BCUS98120/` e descriptografa o EBOOT com o RPCS3 (`RPCS3=<caminho>` se ele não estiver em `/Applications`) |
| `analyse` | `ppu_loader.py` → `analysis/BCUS98120/` |
| `lift` | `ppu_lifter.py` → `recompiled/BCUS98120/` (24 partes; `JOBS` limita o paralelismo) |
| `spu` | `prepare_r2_job_pm.py`, `prepare_r2_bink.py`, `regen_r2_jobmods.sh` → `port/spu/`; `prepare_r2_rawspu.py` → `game/BCUS98120/spu_images/` |
| `build` | CMake + Ninja → `build/BCUS98120/ResistanceRecomp` e `build/BCUS98120/Resistance` |

O primeiro build compila muito código gerado e demora. As ferramentas
precisam de Python 3.10 ou mais novo (`PY=<python>` escolhe um). O disco é
achado pelo `PS3_VFS_ROOT`, por `game/BCUS98120/PS3_GAME` ou por um volume em
`/Volumes` com `BCUS98120` no `PARAM.SFO` (veja `tools/env.sh`).

## Jogando

Monte a imagem do disco (ou deixe-o extraído em `game/BCUS98120/`), conecte
um controle (DualSense, DualShock ou Xbox) e rode:

```sh
tools/play.sh
```

O jogo abre numa janela que pertence ao launcher `Resistance`. Saves, as
pastas de HDD e o log (`last.log`) ficam em
`~/Library/Application Support/resistance-recomp-2/` (`RES_DATA` muda isso).
O título da janela mostra a taxa de quadros. Feche a janela, ou aperte
Ctrl+C no terminal, para sair.

### Ambiente

| Variável | Efeito |
| --- | --- |
| `PS3_VFS_ROOT` | a pasta que tem o `PS3_GAME/` |
| `RES_DATA` | saves, pastas de HDD e o log |
| `RES_FRONTEND_PORT0=0` | desliga o hook de entrada do front end (para comparar) |
| `RES_HTTP_POLL_COMPAT=0` | `cellHttpClientPollConnections` volta ao padrão do toolkit (agora ele responde "indisponível" para a thread HTTP do jogo não girar à toa) |
| `RES_LEVEL=<n>` | passa `-level <n>` para o jogo; ainda não confirmado |
| `PS3_PAD_LOG=0` | desliga o log de botões do controle (ligado no `play.sh`) |
| `PS3_LWCOND_EVENTS=1` | notificações reais de `sys_lwcond` em vez de consulta (experimental) |
| `SPURS_LFQ_NOWAKE=1` | desliga a correção do despertar perdido do SPURS (para comparar) |

### Diagnóstico

O `tools/r2_input_probe.py` roda o build sem janela numa cópia descartável
das suas pastas de HDD, aperta botões a partir de um roteiro (`--buttons`),
salva capturas de tela a cada cinco segundos e grava `probe.json` e
`summary.json` em `build/r2-probe-<rótulo>-*/` (`--help` lista as sondas:
dumps de SPU, amostragem de CPU, estatísticas do SPURS, vigia de memória). O
`tools/compare_r2_logs.py` alinha o log de uma sonda com um log do RPCS3 do
mesmo título.

## Atualizando o patch do toolkit

O port foi desenvolvido numa cópia simples do toolkit. Para transformar as
mudanças de toolkit de uma pasta assim (ou do `./ps3recomp` depois de
editá-lo) de volta no patch:

```sh
tools/export_toolkit_patch.sh /caminho/da/pasta-do-toolkit
```

Ele compara a pasta com um `a6d76c8` limpo (mantendo as mudanças do patch
atual em arquivos que a pasta não tem), deixa de fora o jogo, a tradução, o
código SPU traduzido, os logs e as ferramentas deste repositório, recusa
caminhos absolutos de home e confere que o resultado se aplica num `a6d76c8`
limpo.

## Organização

| Caminho | O que é |
| --- | --- |
| [`port/`](port/) | O port: `main.cpp` (boot, thread do guest), `stubs.cpp`, `patches.cpp` (a chave do hook de entrada do front end, `cellHttpClientPollConnections`), `CMakeLists.txt` (com o alvo da janela `Resistance`), `config.toml`, `spu/` (`spu_workloads.c` e os `register.c` dos módulos SPU crus; o código traduzido deles é gerado ali e nunca vai para o repositório) |
| [`patches/`](patches/) | O patch do toolkit sobre o ps3recomp v0.17.0 |
| [`tools/`](tools/) | `setup_toolkit.sh`, `rebuild.sh`, `play.sh`, `env.sh`, `prepare_r2_job_pm.py`, `prepare_r2_bink.py`, `prepare_r2_rawspu.py`, `regen_r2_jobmods.sh`, `extract_jobmods_elf.py`, `apply_r2_frontend_patch.py`, `r2_input_probe.py`, `compare_r2_logs.py`, `export_toolkit_patch.sh`, `r2_layout.py` |
| [`ROADMAP.pt-BR.md`](ROADMAP.pt-BR.md) | Estado e próximos passos |

## Licença

O código deste repositório: MIT ([LICENSE](LICENSE)). O toolkit é MIT pelos
seus autores. Resistance 2 © Sony Interactive Entertainment LLC,
desenvolvido pela Insomniac Games; este projeto não tem afiliação com a Sony
nem com a Insomniac.
