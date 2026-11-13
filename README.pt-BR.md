# Resistance 2 — recompilação estática para PC (macOS)

<a href="https://recomplauncher.com"><picture><source media="(prefers-color-scheme: dark)" srcset="https://recomplauncher.com/assets/press/launcher/badge-light.svg"><img alt="Get it on RecompLauncher" src="https://recomplauncher.com/assets/press/launcher/badge-dark.svg" height="48"></picture></a>

🌐 [English](README.md) · **Português (BR)**

Port nativo do **Resistance 2** (PS3, disco `BCUS98120`) para
**macOS/arm64**, feito com recompilação estática: o código PowerPC e SPU do
jogo é traduzido para C/C++ e compilado como um executável nativo, com
gráficos em Metal. Não é um emulador.

**A campanha é jogável.** Roda do boot, pelos menus, até a campanha:
Islândia e depois a Bay Area, perto dos 30 fps do próprio jogo num Mac com
Apple silicon (lutas pesadas ainda caem abaixo disso), com controle ou
teclado e mouse. O online pelos servidores da comunidade PS Rewired funciona
pelo launcher com a atualização 01.60 do jogo (login, lobby e uma partida
jogados em 12/11/2026). Veja [Estado](#estado).

Construído sobre o [ps3recomp](https://github.com/sp00nznet/ps3recomp), com o
toolkit, a organização e as ferramentas do
[port do Resistance: Fall of Man](https://github.com/erikomis/resistance-recomp)
do mesmo autor. Veja [CREDITS.pt-BR.md](CREDITS.pt-BR.md).

> [!WARNING]
> **Trabalho em andamento.** A campanha foi jogada até a Bay Area, não até
> o fim, e algumas cenas pesadas rodam abaixo de 30 fps. O launcher
> ([versões](https://github.com/erikomis/resistance2-recomp/releases))
> compila o jogo no seu Mac a partir do seu disco; nada do jogo é baixado.
> Só macOS com Apple silicon. O que falta
> está no [ROADMAP.pt-BR.md](ROADMAP.pt-BR.md).

> **Este repositório não contém dados do jogo, firmware, chaves da Sony nem
> nada traduzido a partir deles.** Cada um gera tudo isso localmente a partir
> da própria cópia legítima. Projeto de fã, sem afiliação com a Sony
> Interactive Entertainment ou a Insomniac Games.

## Estado

Em 30/10/2026: boot → vídeos de abertura → menus → New Game → a primeira
fase (`iceland`) jogada à mão, com combate, diálogos e checkpoints, até a
segunda fase (`bay area`). A gameplay normal roda a **28–30 fps** (eram 14,5
em 29/10); combate pesado ainda pode cair mais (veja os problemas
conhecidos). Sessões de mais de 70 minutos sem travar.

Desde 29/10/2026:

- **Superfícies em cores neon e o mundo deixando de ser desenhado:** o
  backend Metal guardava no máximo 4096 pipelines, e o Resistance 2 vai
  criando pipelines ao longo da fase (mais de 4600 numa sessão de 15
  minutos). Com a tabela cheia, todo pipeline novo falhava e seus desenhos
  eram descartados até o fim da sessão: cores neon chapadas e geometria
  sumindo alguns minutos dentro da fase, pior ao mirar. As tabelas agora têm
  65536 entradas (`PS3RECOMP_METAL_MAX_PIPES`,
  `PS3RECOMP_ENG_MAX_PIPELINES`).
- **Taxa de quadros, 14,5 → ~28 fps:** a thread do RSX estava presa na CPU
  (~68 ms por quadro; a GPU trabalhava ~8 ms). Mapas de sombra e reduções
  de profundidade do NV3089 ficam na GPU em vez de serem lidos de volta a
  cada quadro (`PS3RECOMP_ENG_ZETA16_GPU`); leitura de vértices por
  atributo (upstream #458, `PS3RECOMP_ENG_FETCH_MANY`); um cache de
  vértices que não recomeça mais e confere a geometria residente por
  amostragem (`PS3RECOMP_ENG_VCACHE_KEEP2`, `PS3RECOMP_ENG_VCACHE_SPARSE`);
  decodificação de texturas swizzled mais rápida (upstream #578).
- **Engasgos de shader:** 99% de montar um pipeline era a tradução
  HLSL→MSL, então as traduções ficam em cache no disco entre sessões e
  builds (`PS3_SHADER_CACHE`), os pipelines novos compilam em segundo
  plano, quatro de cada vez (`PS3_SHADER_ASYNC`, `PS3_SHADER_ASYNC_JOBS`), e
  um cuja tradução já está em cache compila na hora
  (`PS3_SHADER_ASYNC_SYNC_HIT`), o que tira o flash rápido ao mirar.
- **Saves:** o jogo salva cada checkpoint com
  `cellSaveDataUserListAutoSave`, e o menu com `UserListSave` /
  `UserListLoad`; nenhuma estava registrada, então o padrão de NID não
  resolvido respondia OK sem gravar nada. Agora elas rodam os callbacks do
  jogo, e os saves vão para a pasta de HDD de onde o jogo os lista.
- **Instalação no HDD:** `cellGameDataCheckCreate2` dizia que a pasta de
  dados do jogo já estava instalada, então o jogo nunca copiava seus caches
  e o diálogo em streaming para o HDD, e toda fala falhava ao abrir. Agora
  responde "instalação nova" até os arquivos existirem; o jogo copia ~354 MB
  no primeiro boot (`PS3_GAMEDATA_ISNEW=0/1` força).
- **Texturas lineares com mips** mantêm o pitch da base em todos os níveis
  (`PS3RECOMP_TEX_LN_PITCH`); uma proteção de recursão que parava um job do
  SPU agora desempilha e retoma.
- **Fundo do menu de pausa e cópias feitas pela CPU:** o backend Metal
  nunca implementou gravar bytes do guest num alvo de cor, então uma cópia
  NV3089 feita pelo caminho da CPU numa memória que a GPU usa como
  superfície se perdia. Na pausa o jogo copia a cena para a memória do
  buffer de profundidade e a desfoca de volta a cada quadro; o menu mostrava
  a profundidade lida como cor (manchas neon) ou um fundo creme liso. Essas
  gravações agora chegam ao alvo na GPU (`PS3RECOMP_ENG_GUEST_WRITE`), e a
  redução de profundidade na GPU voltou a ficar ligada
  (`PS3RECOMP_ENG_BLIT_GPU`).
- **Combate pesado:** a geometria que os SPUs montam cai em endereços novos
  a cada quadro e cada pedaço é desenhado umas duas vezes; o segundo desenho
  agora reaproveita os vértices convertidos no primeiro
  (`PS3RECOMP_ENG_VCACHE_FRAMEREUSE`), o que baixou os quadros acima de
  100 ms de 9 para 5 por minuto num teste.
- **Shaders que amostram textura de volume em coordenadas de texel** tinham
  a coordenada cortada para dois componentes e não compilavam (2400 falhas
  na área Moonpool de uma sessão, com todos esses desenhos descartados);
  passam a ser escalados como volume com `PS3RECOMP_FP_UNNORM_VOL=1` (ligado
  no `play.sh`, sendo confirmado jogando).

Desde 31/10/2026:

- **Continue Game e Load Level:** os saves eram gravados mas nunca lidos de
  volta. `cellSaveDataUserListAutoLoad` não estava registrada, e o código
  provisório que a atendia só rodava o primeiro callback, então o `GAME.SAV`
  nunca era carregado, o menu Campaign só oferecia New Game e Cinematics e
  toda sessão começava do zero. Agora ela roda o carregamento completo
  (`RES_SAVEDATA_LOAD`, `=0` volta ao código provisório): o menu oferece
  Continue Game, New Game, Load Level e Cinematics, e o Continue leva ao
  último checkpoint.
- **Mira:** por alguns quadros depois de apertar a mira, o céu e o fundo
  distante ficavam brancos com faixas de ruído magenta e verde. A transição
  da mira copia a cena em meio tamanho pelo NV3089 da CPU para uma memória
  que também guarda um mapa de sombra Z16 swizzled; nenhuma superfície tinha
  aquele pitch, então o bind da textura tomava o mapa de sombra como o
  conteúdo mais novo, gravava a profundidade por cima da cópia e a decodificava
  como cor. Uma cópia da CPU sobre um alvo de outro layout agora é lembrada e
  vence (`PS3RECOMP_ENG_GUEST_OVER`). Nos testes, 14 de 15 miras piscavam
  antes e 0 de 10 depois.
- **Travadas de 3 segundos ao entrar em áreas novas** ("GPU sit and spin
  timeout"): pipelines com a tradução MSL em disco eram montados na thread do
  RSX, e as compilações do Metal custavam 20-60 ms cada, até 127 seguidas.
  Agora só são montados na hora os pipelines cujas funções já estão na
  memória (`PS3_SHADER_ASYNC_SYNC_HIT=1`).
- **Memória subindo:** a thread do relógio de quadros nunca esvaziava um
  autorelease pool, então cada alvo de renderização criado mantinha um
  descritor de textura vivo (~0,2 MB/s, uns 2 GB em 3 horas). Cada volta agora
  roda dentro de um pool (`PS3_RSX_AUTORELEASE`).
- **Tabela de funções de shader:** as 4096 funções compiladas do backend
  Metal enchiam depois de umas 3 horas, a mesma falha da tabela de
  pipelines; agora são 65536 (`PS3RECOMP_METAL_MAX_FUNCS`).
- **Som estourando:** o runtime oferecia 5.1, então o jogo mixava seis canais
  que depois eram reduzidos para estéreo a até 2,4 vezes o fundo de escala.
  Agora é oferecido estéreo, como no RPCS3, com um limitador para o que ainda
  passa (`PS3_AUDIO_OUT_STEREO`, `PS3_AUDIO_LIMITER`): 3568 amostras cortadas
  num teste de 365 segundos viraram 0.
- **Cópias da cena na GPU:** as cópias de cor do NV3089 são gravadas na GPU
  em vez de lidas de volta (`PS3RECOMP_ENG_BLIT_GPU_COLOR`); as leituras
  caíram de 372 para 93 a cada 120 quadros.
- **Interpretador de SPU:** o laço não grava mais uma global compartilhada
  nem consulta o registro de código traduzido a cada passo
  (`PS3_SPU_INTERP_FAST`): o custo dele caiu de 35-39% para 8-11% das threads
  que o rodam.
- **Trazido do ps3recomp v0.19, desligado enquanto é testado:** os
  modificadores de precisão por fonte nos fragment programs (#670,
  `PS3RECOMP_FP_SRC_PREC`), o limite de profundidade (#693,
  `PS3RECOMP_ENG_DEPTH_CLAMP`), DDX/DDY (#635), o sono da lock-line (#666,
  `SPU_LOCKLINE_SLEEP`), `SPURS_IDLE_MAX` (#659).
- **Superfícies verdes ou com cores erradas no início de uma fase:** os
  desenhos são pulados enquanto o pipeline deles compila em segundo plano.
  Agora todo shader em cache é compilado uma vez em segundo plano assim que o
  jogo abre (`PS3_SHADER_PREWARM`), para o cache de shaders do sistema já
  tê-lo, e os pipelines com tradução em disco voltam a ser montados na hora,
  dentro de um limite de tempo de compilação (`PS3_SHADER_SYNC_BUDGET_MS`,
  8 ms a cada 100 ms), para uma rajada não travar o jogo. Carregando a
  segunda fase com o cache cheio: desenhos pulados 117 mil → 2 mil, mesma taxa
  de quadros, sem travada.
- **Tradução relocável de SPU** (`tools/lift_r2_relocmods.py`, ligada;
  `PS3_RELOCMODS=0` desliga): os 142 módulos SPU independentes de posição
  que os jobs carregam em qualquer lugar do LS eram interpretados.
  Traduzidos, os passos interpretados caem 80-95%. Com a máquina fria a taxa
  de quadros e a CPU não mudam no limite de 30 fps; a carga do interpretador
  quadruplica com a máquina quente, e é aí que a tradução mais economiza.
- **Segunda fase (bay area): engasgos, cores estranhas, água que parecia
  seguir a câmera.** Os programas de pós-processamento e de água amostram
  alvos de renderização em coordenadas de texel, e a escala (1/largura,
  1/altura) ia escrita no texto do shader; na bay area esses alvos mudam de
  tamanho com a vista, então cada tamanho era um shader novo e um pipeline
  novo (37 mil pipelines e 4,3 mil funções Metal numa sessão de 21 minutos,
  254 mil desenhos pulados enquanto os pipelines compilavam). A escala agora
  vai no buffer de constantes (`PS3RECOMP_FP_UNNORM_UNIFORM`). Andando quatro
  minutos pelo corredor alagado: funções 594 → 205, desenhos pulados
  11 mil → 0, passadas de 100 ms ou mais 6 → 1, segundos abaixo de 25 fps
  42 → 0.

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
- **A corrida no FIFO do RSX (travadas de 5 s, geometria explodida, cores
  néon):** o jogo encadeia um buffer lateral cujo JUMP de volta fica em
  `put` e reaproveita essa memória para vértices logo depois de mover o
  `put`. A GPU emulada acordava a cada 1 ms e lia os vértices no lugar do
  JUMP ("FIFO JUMP to unmapped IO", o "GPU sit and spin timeout" de 5
  segundos do jogo, estado perdido na ressincronização). O relógio de
  quadros agora acorda a cada 50 µs (`port/main.cpp`; `PS3_FRAMECLOCK_US`
  ainda ajusta), e o leitor do FIFO relê o `put` a cada 8 palavras e depois
  de cada JUMP/CALL/RET. Um teste de 440 segundos: nenhum salto inválido.
- **Metade esquerda dividida / duplicada:** a cena é desenhada num alvo
  2560×704 (AA 2×1) e a passada final lê 1280×704 dele com coordenadas não
  normalizadas, mas o motor de desenho entregava o alvo inteiro. Agora um
  recorte do tamanho declarado é copiado na GPU. Com essas correções a
  borda clara em volta da imagem jogando também sumiu.
- **Cópias de memória NV0039:** o motor de cópia é implementado no subcanal
  1, onde a libgcm o liga; antes os métodos dele caíam no motor 3D como
  estado.
- **Ordem dos PUTs do SPU:** PUT, PUTLLC e PUTLLUC do SPU escrevem os
  primeiros 128 bytes por último, para quem consulta um cabeçalho nunca o
  ver antes do conteúdo (port do
  [#632](https://github.com/sp00nznet/ps3recomp/pull/632) do ps3recomp).
- **Controle na primeira tela:** o despachante de entrada do front end lia o
  port 0 como desabilitado antes de existir um jogador local, então o X não
  fazia nada no aviso de auto-save. Um hook bem restrito no despachante
  traduzido (`tools/apply_r2_frontend_patch.py`, aplicado na configuração do
  CMake; chave: `RES_FRONTEND_PORT0=0`) mantém os eventos do port 0
  chegando. Ele não injeta nada.

Problemas conhecidos, em andamento:

- **Combate pesado** ainda pode cair abaixo de 30 fps: a geometria dos SPUs
  em endereços novos a cada quadro erra o cache de vértices (o
  reaproveitamento no mesmo quadro, acima, corta metade da conversão; falta
  mais).
- **Aquecimento:** o jogo usa uns 3,5 núcleos de CPU a 30 fps; quando o
  macOS acusa pressão térmica, a taxa pode cair para 11-19 fps por dezenas de
  segundos. A tradução relocável de SPU acima corta a parte do interpretador
  nessa carga; o efeito com a máquina quente ainda não foi medido.
- **Efeitos** a confirmar jogando depois das correções acima: quadrados
  pretos no lugar de vapor/água, faíscas elétricas estranhas, uma distorção
  como água que acompanha a câmera em alguns lugares (área Moonpool).
- A neblina sai um pouco mais escura que no PS3.

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
instância, aposentadoria de overlays de SPU velhos, o motor de cópia NV0039,
a releitura do `put` no leitor do FIFO do RSX, a amostragem de um recorte de
um alvo de cor maior, PUTs do SPU com o cabeçalho escrito por último, `ESRCH` para threads que
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

O jeito mais fácil é o launcher em [`launcher/`](launcher/README.md): ele
instala e recompila o port a partir do seu disco, guarda as configurações e
os saves e abre o jogo sem terminal. Pelo terminal:

Monte a imagem do disco (ou deixe-o extraído em `game/BCUS98120/`), conecte
um controle (DualSense, DualShock ou Xbox) e rode:

```sh
tools/play.sh
```

O jogo abre numa janela que pertence ao launcher `Resistance`. Saves, as
pastas de HDD e o log (`last.log`) ficam em
`~/Library/Application Support/resistance-recomp-2/` (`RES_DATA` muda isso).
Os 10 últimos logs de sessão vão para `logs/`, e cada linha do log começa
com os segundos desde o início. O título da janela mostra a taxa de
quadros. Feche a janela, ou aperte Ctrl+C no terminal, para sair. O
primeiro boot copia ~354 MB de dados do jogo para a pasta de HDD, como no
PS3.

### Jogando online

O online passa pelo PS Rewired, um servidor da comunidade que mantém o
multiplayer do Resistance 2 funcionando. Ele precisa da última atualização
do jogo, a 01.60, como um segundo build ao lado do 01.00, que fica com a
campanha:

```sh
tools/build_online.sh       # download extract stage decrypt analyse lift spu build
```

Ele baixa os dois pacotes oficiais da atualização (01.50 e 01.60, uns
340 MB) do servidor de atualizações da Sony, confere o tamanho e o SHA-1,
extrai, decifra o `EBOOT.BIN` da 01.60 e compila em
`build/BCUS98120_v160/` (qualquer passo roda sozinho pelo nome, como no
`rebuild.sh`; sem nenhum nome ele continua de onde parou e pula o que já
está feito). O `EBOOT.BIN` da 01.60 é um arquivo NPDRM de licença gratuita,
então o `tools/decrypt_self.py` o decifra sem o RPCS3, com chaves que o
`tools/fetch_keys.py --npdrm` tira do código-fonte público do RPCS3
(conferido por SHA-256, guardado fora do repositório); o RPCS3 só entra
quando isso falha. O que ele escreve vai para `game/BCUS98120_v160/`,
`analysis/`, `recompiled/`, `build/` e `port/spu_v160/`, tudo no
`.gitignore`.

Depois, no launcher, digite o seu online ID (de 3 a 16 letras, números, `-`
ou `_`, começando por letra) e aperte **Jogar online**. Ele roda o build
01.60 numa pasta própria (`online/` dentro da pasta de jogo, com pastas de
HDD próprias, então a instalação e os saves da campanha nunca são tocados;
ele parte de uma cópia da instalação no HD da campanha, então jogue a
campanha uma vez antes), e o jogo só alcança os endereços do PS Rewired. O
online roda sempre nos 30 fps do próprio jogo: as regras do servidor
proíbem aumentar ou limitar a taxa de quadros, então o launcher deixa de
fora toda chave de taxa de quadros e de tempo, das configurações e das
variáveis extras. Quando você entra no online pelo menu do jogo, a primeira
tela do servidor é o contrato de uso: aceitar ou recusar é com você. Em
12/11/2026 o build 01.60 abre, faz login e chega a essa tela; o lobby, a
lista de partidas e uma partida ainda não foram testados.

**Pelo RecompLauncher**: a instalação dele compila também a versão online,
logo depois da campanha (uns 30 minutos e 4 GB a mais, com o download de
340 MB da Sony). Se essa parte falhar, a campanha continua pronta, e a
última linha da instalação diz o porquê. O jogo fica com dois blocos,
"Resistance 2" e "Resistance 2 — Online (PS Rewired)". O RecompLauncher não
tem onde digitar texto, então defina o seu online ID uma vez na janela do
Resistance 2 Launcher (Início > Online; o botão **Resistance 2 Launcher** na
página do jogo no RecompLauncher a abre). Aí o bloco Online abre o online
direto, sempre a 30 fps; se faltar algo (o ID, a versão online, a primeira
vez da campanha) ele diz o quê. Detalhes e códigos de saída:
[launcher/README.md](launcher/README.md#recomplauncher).

### Ambiente

| Variável | Efeito |
| --- | --- |
| `PS3_VFS_ROOT` | a pasta que tem o `PS3_GAME/` |
| `RES_DATA` | saves, pastas de HDD e o log |
| `RES_FRONTEND_PORT0=0` | desliga o hook de entrada do front end (para comparar) |
| `RES_HTTP_POLL_COMPAT=0` | `cellHttpClientPollConnections` volta ao padrão do toolkit (agora ele responde "indisponível" para a thread HTTP do jogo não girar à toa) |
| `RES_LEVEL=<n>` | passa `-level <n>` para o jogo; ainda não confirmado |
| `PS3_PAD_LOG=0` | desliga o log de botões do controle (ligado no `play.sh`) |
| `PS3_PAD_ADAPTIVE=0` / `PS3_PAD_FIRE_AT=<10-90>` | desliga os gatilhos adaptáveis do DualSense / onde o R2 conta como apertado, em porcentagem do curso (50). O Steam com o Steam Input ligado toma conta do DualSense e os gatilhos e a luz ficam desligados: feche o Steam ou desligue o suporte a controles PlayStation nele (o log avisa quando o Steam está aberto) |
| `PS3_PAD_LIGHT=<RRGGBB>\|off`, `PS3_PAD_LIGHT_HIT=0` | a barra de luz do DualSense / DualShock 4 (o laranja do Resistance 2, `ff5a00`, por padrão) / sem o brilho vermelho com a vibração do jogo |
| `PS3_PAD_GYRO=<0-400>`, `PS3_PAD_GYRO_MODE=always`, `PS3_PAD_GYRO_INVERT_Y=1` | mira por giroscópio em porcentagem (desligada por padrão): girar e inclinar o controle soma ao analógico direito enquanto mira (L1, onde os controles o colocarem), ou sempre |
| `PS3_FRAMEGEN=1` | geração de quadros MetalFX (macOS 26+; um botão no launcher): um quadro gerado entre cada dois que o jogo desenha, sem mexer no HUD. Experimental, desligada por padrão; veja abaixo |
| `PS3_FRAMEGEN_VPCONST`, `PS3_FRAMEGEN_HUD_AFTER`, `PS3_FRAMEGEN_STATIC_BELOW` | onde a geração de quadros acha a câmera do Resistance 2 (constantes de vértice c[8..11]), o fim da imagem 3D (fragment program `23c9cdc5b534e1c9`) e a arma em primeira pessoa (profundidade abaixo de 0,2); o port já define, mude só para testes |
| `GCM_STATE_SHADOW=1`, `PS3RECOMP_ENG_TEXSLOT_INDEX=0`, `PS3RECOMP_ENG_VSCB_SYNC=0`, `PS3RECOMP_ENG_ADDR_FILTER=0`, `PS3RECOMP_METAL_MAX_VIEWS=256`, `PS3RECOMP_METAL_VIEW_EVICT=0`, `PS3RECOMP_ENG_ZSNAP_OWN=0`, `GCM_FAST_WORDS=0` | desligam, um a um, os ganhos da thread do RSX vindos do port do Resistance 3 (sem a sombra de estado antiga, índice hash para achar texturas, só as constantes de vértice que mudaram copiadas, filtro de endereços antes das buscas de superfície, 2048 visões de textura com descarte LRU, o snapshot da GPU para a profundidade lida do próprio zeta, caminho direto para palavras NV4097 simples) |
| `PS3_TB_POLL_SLEEP_US=<µs>` (0 = desligado, padrão 10), `PS3_TB_POLL_CALLS`, `PS3_TB_POLL_GAP_NS` | uma thread que só lê o relógio (o limitador de quadros do jogo esperando) dorme um pouco a cada leitura depois de 2000 leituras vazias, em vez de segurar um núcleo a 100% (vindo do port do Resistance 3) |
| `PS3_DOWNSCALE=0` | uma resolução interna acima da janela vai para a janela como está e a camada encolhe por amostragem (o serrilhado e a cintilação ficam); por padrão cada quadro é reduzido para a janela com um filtro Lanczos (MetalPerformanceShaders), contando todos os pixels: supersampling (vindo do port do Resistance 1) |
| `PPU_STWCX_SAME_NOTIFY=1`, `SPU_WAKE_LOCK=0`, `SPU_WAIT_SPIN_US=<µs>` (0 = desligado, padrão 100), `SPURS_RCS_FAST=0`, `SPU_BRANCH_ALIGN=0`, `PS3RECOMP_FP_ALPHA_QUANT=0` | desligam, uma a uma, as correções do port para PC do Resistance de Psyykosis que vieram pelo port do Resistance 1 (um `stwcx.` com o mesmo valor mantendo as reservas das SPUs, o "acordar" do canal sob a trava, a espera curta das SPUs raw antes de dormir, o `cellSpursReadyCountStore` sem trava, os saltos indiretos das SPUs alinhados, o teste de alfa sobre o alfa de 8 bits guardado) |
| `PS3_LWCOND_EVENTS=0` | `sys_lwcond` volta a dormir e tentar de novo (as notificações reais são o padrão) |
| `SPURS_LFQ_NOWAKE=1` | desliga a correção do despertar perdido do SPURS (para comparar) |
| `SPU_OVL_SIG_OFF=1` / `SPU_OVL_SIG_ONLY=<ids>` | roda os módulos de job interpretados / só estes traduzidos (bissecção) |
| `PS3_FRAMECLOCK_US=<µs>` | o período do relógio de quadros / do RSX (padrão 50; 1000 traz de volta a corrida no FIFO) |
| `GCM_PUT_LIVE=0` | o leitor do FIFO para de reler o `put` no meio da leitura |
| `GCM_SUBCH1_M2MF=0` | o subcanal 1 volta ao motor 3D em vez do motor de cópia NV0039 |
| `GCM_FIFO_STRADDLE=1` | proteção opcional para uma leitura do FIFO que atravessa o `put` (diagnóstico) |
| `PS3RECOMP_ENG_SUBRECT=0` | desliga a amostragem de recorte de um alvo de cor maior (traz de volta a imagem dividida) |
| `PS3_TELEMETRY=0` | desliga a linha `[tele]` por segundo (quadros, desenhos) e o relatório de travada de 3 s (ligado no `play.sh`) |
| `RSX_DRAIN_SPIKE_MS=<ms>` | relata cada passada do FIFO com pelo menos esse tempo e no que foi gasta (100 no `play.sh`) |
| `PS3_SHADER_CACHE=<pasta>` / `PS3_SHADER_ASYNC=0` | o cache de tradução de shaders (em `RES_DATA/shadercache`) / pipelines compilados em segundo plano (os dois ligados no `play.sh`) |
| `PS3_SHADER_ASYNC_JOBS=<n>`, `PS3_SHADER_ASYNC_SYNC_HIT=0/1/2` | compilações paralelas em segundo plano (4); montar na hora quando as funções estão na memória (1) ou também do cache em disco dentro do limite (2, o padrão) |
| `PS3_SHADER_PREWARM=0`, `PS3_SHADER_SYNC_BUDGET_MS=<ms>` | sem a compilação em segundo plano dos shaders em cache ao abrir; tempo de compilação permitido a cada 100 ms para montagens feitas na hora a partir do disco (8) |
| `PS3RECOMP_ENG_GUEST_OVER=0`, `PS3_RSX_AUTORELEASE=0`, `PS3RECOMP_ENG_BLIT_GPU_COLOR=0` | desligam a correção da mira, o autorelease pool, as cópias da cena na GPU |
| `PS3_AUDIO_OUT_STEREO=0`, `PS3_AUDIO_LIMITER=0`, `PS3_SPU_INTERP_FAST=0` | voltam a oferecer 5.1, tiram o limitador, voltam ao laço antigo do interpretador |
| `PS3_RELOCMODS=0` | deixa os módulos SPU independentes de posição no interpretador |
| `PS3RECOMP_FP_UNNORM_UNIFORM=0` | volta a escrever a escala de texel dentro de cada shader (traz de volta a explosão de shaders da bay area) |
| `RES_SAVEDATA_LOAD=0` | o save deixa de ser lido no boot (sem Continue Game) |
| `PS3RECOMP_METAL_MAX_PIPES=<n>`, `PS3RECOMP_ENG_MAX_PIPELINES=<n>` | tamanho das tabelas de pipeline (65536; 4096/8192 trazem o neon de volta) |
| `PS3RECOMP_ENG_ZETA16_GPU=0`, `PS3RECOMP_ENG_FETCH_MANY=0`, `PS3RECOMP_ENG_VCACHE_SPARSE=0` | desligam as mudanças de desempenho acima, uma de cada vez |
| `PS3RECOMP_ENG_BLIT_GPU=0`, `PS3RECOMP_ENG_GUEST_WRITE=0` | desligam a redução de profundidade na GPU / as gravações da CPU em alvos da GPU (ligadas por padrão; desligar traz de volta o menu de pausa quebrado) |
| `PS3RECOMP_ENG_VCACHE_FRAMEREUSE=0`, `PS3RECOMP_ENG_VCACHE_CHURN=0` | desligam o reaproveitamento de vértices no mesmo quadro / o descanso de buffers que mudam sempre (ligados por padrão) |
| `PS3RECOMP_FP_UNNORM_VOL=0/1` | texturas de volume em coordenadas de texel escaladas como volume (ligado no `play.sh`) |
| `PS3RECOMP_ENG_PLANAR=1`, `PS3RECOMP_METAL_TARGET_POOL=1` | medidos e descartados (mais lentos): um fluxo de vértices por atributo, um pool reaproveitado de alvos de sombra |
| `PS3_GAMEDATA_ISNEW=0/1` | força o que o jogo ouve sobre a instalação no HDD |

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

A geração de quadros tem um autoteste: `PS3_FRAMEGEN=1
PS3_FRAMEGEN_TEST=<pasta>` numa rodada sem janela refaz um quadro sim, outro
não, a partir dos vizinhos e grava ao lado do real (`PS3_FRAMEGEN_TEST_AT=<s>`,
`_EVERY`, `_MAX`, `_SCALES=0.5,1.5,2` para variar a escala do movimento), e o
`tools/framegen_score.py <pasta>` dá a nota. Andando e virando na primeira
fase: 34–41 dB contra 24–28 de uma mistura 50/50 e 27–33 sem movimento, escala
1 a melhor, câmera parada com movimento exatamente zero, ~2 ms de GPU por
quadro gerado em 720p. `PS3_FG_VPDUMP=<pasta>` (com `_AT`, `_FRAMES`) grava
alguns quadros de desenhos com as constantes de vértice e os programas; foi
assim que a câmera foi achada.

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
| [`tools/`](tools/) | `setup_toolkit.sh`, `rebuild.sh`, `build_online.sh`, `play.sh`, `env.sh`, `prepare_r2_job_pm.py`, `prepare_r2_rawspu.py`, `prepare_r2_rawspu_loader.py`, `regen_r2_jobmods.sh`, `extract_jobmods_elf.py`, `apply_r2_frontend_patch.py`, `r2_input_probe.py`, `compare_r2_logs.py`, `framegen_score.py`, `export_toolkit_patch.sh`, `r2_layout.py` |
| [`ROADMAP.pt-BR.md`](ROADMAP.pt-BR.md) | Estado e próximos passos |

## Licença

O código deste repositório: MIT ([LICENSE](LICENSE)). O toolkit é MIT pelos
seus autores. Resistance 2 © Sony Interactive Entertainment LLC,
desenvolvido pela Insomniac Games; este projeto não tem afiliação com a Sony
nem com a Insomniac.
