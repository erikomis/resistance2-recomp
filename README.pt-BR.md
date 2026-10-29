# Resistance 2 — recompilação estática para PC (macOS)

🌐 [English](README.md) · **Português (BR)**

Port nativo do **Resistance 2** (PS3, disco `BCUS98120`) para
**macOS/arm64**, feito com recompilação estática: o código PowerPC e SPU do
jogo é traduzido para C/C++ e compilado como um executável nativo, com
gráficos em Metal. Não é um emulador.

**Ele chega à gameplay controlável na primeira fase.** O jogo roda do boot
e do aviso de auto-save, pelos vídeos de abertura (nos seus 30 fps) e pelos
menus, até o New Game, toca a cinemática da fase na engine e passa para a
gameplay em primeira pessoa com o HUD na primeira fase (`iceland`): o
jogador anda. Roda a uns 18 fps jogando, com quedas, e a imagem ainda não é
composta direito; veja [Estado](#estado).

Construído sobre o [ps3recomp](https://github.com/sp00nznet/ps3recomp), com o
toolkit, a organização e as ferramentas do
[port do Resistance: Fall of Man](https://github.com/erikomis/resistance-recomp)
do mesmo autor. Veja [CREDITS.pt-BR.md](CREDITS.pt-BR.md).

> [!WARNING]
> **Trabalho em andamento, no começo.** Dá para entrar na primeira fase e
> andar nela, mas a uns 18 fps e com erros de imagem; ainda não é um jeito
> acabado de jogar. Não há releases: você mesmo compila o jogo a partir do
> seu disco. Só macOS com Apple silicon, com controle. O que falta está no
> [ROADMAP.pt-BR.md](ROADMAP.pt-BR.md).

> **Este repositório não contém dados do jogo, firmware, chaves da Sony nem
> nada traduzido a partir deles.** Cada um gera tudo isso localmente a partir
> da própria cópia legítima. Projeto de fã, sem afiliação com a Sony
> Interactive Entertainment ou a Insomniac Games.

## Estado

Em 29/10/2026: boot → aviso de auto-save → vídeos de abertura a 30 fps →
título e menus (Campaign → New Game → dificuldade) → a primeira fase
(`iceland`) carrega → cinemática na engine → **gameplay em primeira pessoa
com HUD**, o jogador anda (um teste roteirizado de 420 segundos levou o
marcador de objetivo de 23 m para 12 m). Média de uns 18 fps jogando, com
quedas.

O que foi preciso, com a causa de cada um:

- **Layout do r4 do SPURS:** o módulo de política Wws_Job usa o layout
  canônico do r4 (word 1 = a lista de jobs). Pôr a lista no word 0 fazia ele
  ler a lista como buffer de auditoria, escrever `FFFFFFFF` por cima e sair
  sem rodar job: o travamento "render db spu timeout in WaitTillReady" aos
  ~90 s. O port não define mais `SPURS_PM_R4_WORD0`.
- **`ESRCH` para threads que acabaram:** o sistema de save espera a sua
  "Check Thread" sumir consultando `sys_ppu_thread_get_priority` até receber
  `ESRCH`; o runtime respondia OK para threads encerradas e o "New Game"
  travava.
- **Programas de raw SPU traduzidos:** o decodificador Bink e os dois
  programas que as fases rodam a cada quadro são bytes crus de SPU (cabeçalho
  `DEADDEAD`, não ELFs) que o `igInitRawSpu` carrega em LS `0x200`. O
  scanner de imagens nunca os via e eles rodavam no interpretador (vídeos a
  ~3 fps). O `tools/prepare_r2_rawspu.py` traduz os três; eles são
  registrados como regiões de código residentes pelo endereço de origem. O
  módulo de política Wws_Job (também bytes crus) é traduzido pelo
  `tools/prepare_r2_job_pm.py`.
- **Despertar perdido na LFQueue:** um travamento intermitente ao carregar a
  fase, entre o push do PPU numa fila lock-free e o `WAIT_SIGNAL` da tarefa
  zlib. A tarefa agora se registra como "parked" antes de reverificar a
  fila, com barreiras dos dois lados.
- **IDs de workload por instância:** o jogo usa duas instâncias SPURS; o
  runtime numerava os workloads com um contador global de 16, então o 17º
  ("triangulate") ficava sem ID e a API de jobs disparava um assert. As
  tabelas de workloads do `cellSpurs.c` agora são por instância.
- **29 módulos de job traduzidos:** os módulos de job WWS (cabeçalhos
  `0xC0DEC0DE` no EBOOT, carregados em LS `0x4000`) são traduzidos pelo
  `tools/regen_r2_jobmods.sh` e registrados pela assinatura.
- **Overlay velho aposentado:** o gerenciador de jobs também carrega em
  `0x4000` corpos que não estão entre esses 29; o runtime mantinha o módulo
  anterior "residente" e rodava o código traduzido dele sobre os bytes novos
  (geometria corrompida no jogo). Agora o overlay é aposentado quando o
  cabeçalho dele no LS é sobrescrito (chaves de A/B: `SPU_OVL_SIG_OFF=1`,
  `SPU_OVL_SIG_ONLY=<ids>`).
- **Eventos de lwcond ligados por padrão:** notificações reais de
  `sys_lwcond` (`PS3_LWCOND_EVENTS=1`, agora o padrão no `port/main.cpp`)
  baixaram a pausa depois do vídeo da Insomniac de ~5 s para ~1 s.
- **Controle na primeira tela:** o despachante de entrada do front end lia o
  port 0 como desabilitado antes de existir um jogador local, então o X não
  fazia nada no aviso de auto-save. Um hook bem restrito no despachante
  traduzido (`tools/apply_r2_frontend_patch.py`, aplicado na configuração do
  CMake; chave: `RES_FRONTEND_PORT0=0`) mantém os eventos do port 0
  chegando. Ele não injeta nada.

Problemas conhecidos, em andamento:

- **Desempenho na fase (~18 fps, com quedas):** código SPU que a fase copia
  dos seus dados para endereços variáveis do LS ainda roda no interpretador
  (como os módulos relocáveis do inFAMOUS 2: precisa de uma tradução
  relocável), e os shaders compilam de forma síncrona.
- **Menus e cinemáticas:** a metade esquerda da tela é composta errada (um
  buffer de 2560 de largura / pós-processamento).
- **Jogando:** a imagem ocupa uns 1152×654 da janela, com uma borda clara à
  direita e embaixo (viewport / escala do buffer de exibição).
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
   do EBOOT `0x00CF9680`, LS `0xA00`), os três programas de raw SPU (o
   decodificador Bink em `0x00EB6580` e dois programas das fases em
   `0x00E9B400` e `0x00EA2580`, LS `0x200`), o carregador `igInitRawSpu`
   (`0x00ED2330`, LS `0x10`) e os 29 módulos de job WWS. Os scripts `tools/prepare_r2_*.py` e o
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
correção do despertar perdido no SPURS, tabelas de workloads do SPURS por
instância, aposentadoria de overlays de SPU velhos, `ESRCH` para threads que
acabaram, notificações reais de `sys_lwcond`, um timebase ancorado na
primeira leitura e o log de controle com `PS3_PAD_LOG=1`).

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
| `spu` | `prepare_r2_job_pm.py`, `prepare_r2_rawspu.py`, `regen_r2_jobmods.sh` → `port/spu/`; `prepare_r2_rawspu_loader.py` → `game/BCUS98120/spu_images/` |
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
| `PS3_LWCOND_EVENTS=0` | `sys_lwcond` volta a dormir e tentar de novo (as notificações reais são o padrão) |
| `SPURS_LFQ_NOWAKE=1` | desliga a correção do despertar perdido do SPURS (para comparar) |
| `SPU_OVL_SIG_OFF=1` / `SPU_OVL_SIG_ONLY=<ids>` | roda os módulos de job interpretados / só estes traduzidos (bissecção) |

### Diagnóstico

O `tools/r2_input_probe.py` roda o build sem janela numa cópia descartável
das suas pastas de HDD, aperta botões a partir de um roteiro (`--buttons`),
salva capturas de tela a cada cinco segundos e grava `probe.json` e
`summary.json` em `build/r2-probe-<rótulo>-*/` (`--help` lista as sondas:
dumps de SPU, amostragem de CPU, estatísticas do SPURS, vigia de memória;
`--env CHAVE=VALOR` acrescenta qualquer variável). Até a primeira fase e
andando, como foi testado:

```sh
B="12:0x4000,20:0x4000"; for t in $(seq 30 8 110); do B="$B,$t:0x4008"; done
python3 tools/r2_input_probe.py --label level --buttons "$B" --seconds 320
```

O `tools/compare_r2_logs.py` alinha o log de uma sonda com um log do RPCS3
do mesmo título.

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
| [`port/`](port/) | O port: `main.cpp` (boot, thread do guest), `stubs.cpp`, `patches.cpp` (a chave do hook de entrada do front end, `cellHttpClientPollConnections`), `CMakeLists.txt` (com o alvo da janela `Resistance`), `config.toml`, `spu/` (`spu_workloads.c` e o `register.c` do módulo de política; o código traduzido dos módulos SPU crus e dos módulos de job, e os `register.c` dos programas de raw SPU, são gerados ali e nunca vão para o repositório) |
| [`patches/`](patches/) | O patch do toolkit sobre o ps3recomp v0.17.0 |
| [`tools/`](tools/) | `setup_toolkit.sh`, `rebuild.sh`, `play.sh`, `env.sh`, `prepare_r2_job_pm.py`, `prepare_r2_rawspu.py`, `prepare_r2_rawspu_loader.py`, `regen_r2_jobmods.sh`, `extract_jobmods_elf.py`, `apply_r2_frontend_patch.py`, `r2_input_probe.py`, `compare_r2_logs.py`, `export_toolkit_patch.sh`, `r2_layout.py` |
| [`ROADMAP.pt-BR.md`](ROADMAP.pt-BR.md) | Estado e próximos passos |

## Licença

O código deste repositório: MIT ([LICENSE](LICENSE)). O toolkit é MIT pelos
seus autores. Resistance 2 © Sony Interactive Entertainment LLC,
desenvolvido pela Insomniac Games; este projeto não tem afiliação com a Sony
nem com a Insomniac.
