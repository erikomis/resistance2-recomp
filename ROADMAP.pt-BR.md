# Roadmap

🌐 [English](ROADMAP.md) · **Português (BR)**

Estado do port nativo do Resistance 2 (`BCUS98120`) para macOS/arm64 e o que
vem a seguir. Mesma organização do roadmap do Resistance: Fall of Man.
`[x]` = feito e verificado; `[ ]` = a fazer; "(em andamento)" = sendo feito.

## Resumo (29/10/2026)

- **Gameplay controlável na primeira fase:** boot → menus → New Game →
  vídeos de abertura a 30 fps → cinemática na engine → gameplay em primeira
  pessoa com HUD em `iceland`; o jogador anda. A imagem de tela cheia está
  correta (sem divisão, sem borda) e um teste de 440 segundos não teve
  travada da GPU.
- **Uns 15–19 fps jogando:** código SPU relocado ainda interpretado,
  compilação síncrona de shaders.

## Feito

- [x] O toolkit: o ps3recomp v0.17.0 do upstream mais o patch de toolkit do
  Resistance: Fall of Man, num patch só com as mudanças do Resistance 2
  (`patches/ps3recomp-a6d76c8-resistance2.patch`).
- [x] EBOOT descriptografado, analisado e traduzido (umas 41.500 funções, 24
  unidades de tradução); compila e linka no macOS/arm64 com Metal.
- [x] Os ELFs de SPU embutidos traduzidos pelo build; o carregador do raw SPU
  (`igInitRawSpu`) recortado do EBOOT e traduzido junto.
- [x] O módulo de política SPU Wws_Job (bytes crus em `0x00CF9680` do EBOOT)
  traduzido e registrado pela assinatura; o layout canônico do r4 do SPURS
  (o travamento "WaitTillReady").
- [x] Controle no aviso de auto-save: o hook do port 0 do front end
  (`tools/apply_r2_frontend_patch.py`, `RES_FRONTEND_PORT0`).
- [x] `cellHttpClientPollConnections` responde "indisponível" em vez de um
  sucesso falso, e a thread HTTP do jogo espera em vez de girar à toa.
- [x] Os três programas de raw SPU traduzidos (`tools/prepare_r2_rawspu.py`):
  o decodificador Bink (vídeos a 30 fps) e os dois programas das fases.
- [x] Travamento do "New Game": `ESRCH` para uma thread que já acabou.
- [x] Despertar perdido na LFQueue no `WAIT_SIGNAL` (travamento intermitente
  ao carregar a fase).
- [x] IDs de workload do SPURS por instância (duas instâncias; o 17º
  workload, "triangulate", ficava sem ID).
- [x] Os 29 módulos de job WWS traduzidos (`tools/regen_r2_jobmods.sh`).
- [x] Overlays velhos de módulos de job aposentados quando o cabeçalho no LS
  é sobrescrito (geometria corrompida no jogo).
- [x] Notificações reais de `sys_lwcond` por padrão (a pausa depois do vídeo
  de ~5 s para ~1 s).
- [x] Entrada na primeira fase (`iceland`): cinemática, HUD, o jogador anda
  (teste roteirizado de 420 segundos).
- [x] A corrida no FIFO do RSX: o relógio de quadros / do RSX a 50 µs (era
  1 ms) e o `put` relido no meio da leitura ("FIFO JUMP to unmapped IO", o
  "GPU sit and spin timeout" de 5 segundos, geometria corrompida); nenhum
  salto inválido em 440 s.
- [x] A metade esquerda dividida / duplicada: amostragem de recorte do alvo
  AA 2560×704; a borda clara jogando sumiu.
- [x] O motor de cópia de memória NV0039 no subcanal 1.
- [x] PUTs do SPU escrevem os primeiros 128 bytes por último (upstream #632,
  portado).
- [x] A primeira fase jogada inteira à mão, com combate e diálogos, até a
  segunda fase (`bay area`); sessões de 70 minutos sem travar.
- [x] Superfícies neon e geometria sumindo minutos dentro da fase: a tabela
  de 4096 pipelines enchia e os desenhos de pipelines novos eram
  descartados; agora 65536.
- [x] Gameplay normal de ~15 para 28–30 fps: mapas de sombra e reduções de
  profundidade na GPU, leitura de vértices por atributo (upstream #458), um
  cache de vértices que guarda e confere por amostragem, decodificação
  swizzled mais rápida (upstream #578).
- [x] Engasgos de shader: traduções em cache entre sessões e builds, quatro
  compilações em segundo plano, as em cache compiladas na hora (sem flash ao
  mirar).
- [x] Saves: `cellSaveDataUserListAutoSave` / `UserListSave` /
  `UserListLoad` implementadas (antes nada era gravado).
- [x] Instalação no HDD: o jogo copia seus caches e o diálogo em streaming no
  primeiro boot (`cellGameDataCheckCreate2` responde instalação nova).
- [x] Logs das sessões: linhas com tempo, as 10 últimas guardadas, uma linha
  de telemetria por segundo, relatório de travada de 3 s e detalhe das
  passadas longas.
- [x] Fundo do menu de pausa: cópias NV3089 feitas pela CPU em alvos da GPU
  agora chegam à GPU (`PS3RECOMP_ENG_GUEST_WRITE`); redução de profundidade
  na GPU religada.
- [x] Reaproveitamento de vértices no mesmo quadro para a geometria dos SPUs
  desenhada duas vezes (`PS3RECOMP_ENG_VCACHE_FRAMEREUSE`).
- [x] Corrigidos: o piscar da mira (uma cópia da cena pela CPU sobre a
  memória de um mapa de sombra); as travadas de 3 segundos ao entrar em áreas
  novas (compilações do Metal na thread do RSX); a memória subindo (um
  autorelease pool na thread do relógio de quadros); tabela de 65536 funções
  de shader; saída estéreo com limitador (sem estouro); cópias da cena na GPU;
  um laço mais rápido no interpretador de SPU; encerramento limpo ao fechar a
  janela e com SIGTERM; caminhos de HDD no cellFs e nos troféus; nenhum
  desenho descartado além de 8192 registros.
- [x] Medidos e descartados: um fluxo de vértices por atributo
  (`PS3RECOMP_ENG_PLANAR`), um pool reaproveitado de alvos de sombra.
- [x] Gatilhos adaptáveis do DualSense, vindos do port do Resistance 1
  (11/11/2026): o R2 tem o clique de um gatilho de arma e atira no clique
  (mais ou menos na metade), vibra enquanto está apertado e o jogo vibra; o
  L2 tem uma resistência leve. Os outros controles continuam com gatilhos
  livres. `PS3_PAD_ADAPTIVE=0` desliga, `PS3_PAD_FIRE_AT` muda o ponto do
  aperto; um botão no launcher, e um botão Atirar com R2 (estilo PS5) na
  página Controles que troca L1/R1 com L2/R2. Ainda não sentido num DualSense
  de verdade.
- [x] Geração de quadros MetalFX, vinda do port do inFAMOUS 2 (12/11/2026,
  `PS3_FRAMEGEN=1`, um botão no launcher, desligada por padrão, macOS 26+):
  um quadro gerado entre cada dois que o jogo desenha, com o HUD recolocado
  intacto. A câmera é a view-projection do mundo nas constantes de vértice
  c[8..11] (a mais comum entre os desenhos 3D do quadro, o que deixa de fora
  as cascatas de sombra e a projeção própria da arma em primeira pessoa); a
  profundidade é copiada onde a cena é composta; a arma (profundidade abaixo
  de 0,2) anda junto com a câmera. Autoteste sem janela andando e virando:
  34–41 dB contra 24–28 de uma mistura 50/50, escala de movimento 1 a melhor,
  câmera parada com movimento exatamente zero, ~2 ms de GPU por quadro
  gerado. Ligada também pelo host de janela (o Jogar do launcher). Ainda
  não testada numa janela por um jogador.

- [x] Mais do DualSense (13/11/2026): a barra de luz no laranja do jogo (ou
  outra cor, ou desligada) ficando vermelha com tiros e explosões, as luzes
  de jogador e a mira por giroscópio opcional (girar e inclinar o controle
  segurando o gatilho esquerdo, ou sempre). Os gatilhos adaptáveis não faziam
  nada no DualSense do jogador porque o Steam (Steam Input) segurava o
  controle e reescrevia a saída dele; com o Steam fechado eles funcionam. O
  port agora limpa os bits de economia de energia do controle ao abri-lo e
  avisa no log quando o Steam está aberto. Mira por giroscópio ainda não
  testada por um jogador.

## Próximos passos

- [ ] (em andamento) Combate pesado abaixo de 30 fps: a geometria dos SPUs
  em endereços novos a cada quadro erra o cache de vértices.
- [ ] (em andamento) Confirmar jogando: os efeitos da Moonpool (quadrados
  pretos no lugar de vapor/água, faíscas estranhas, distorção como água
  acompanhando a câmera) depois de `PS3RECOMP_FP_UNNORM_VOL` e
  `PS3RECOMP_ENG_GUEST_WRITE`; neblina um pouco mais escura que no PS3.
- [x] Saves entre sessões: os checkpoints são gravados, e o Continue Game
  carrega o último (`cellSaveDataUserListAutoLoad`).
- [x] Tradução relocável de SPU ligada por padrão
  (`tools/lift_r2_relocmods.py`).
- [x] Explosão de shaders na bay area: a escala de texel vai no buffer de
  constantes (`PS3RECOMP_FP_UNNORM_UNIFORM`).
- [ ] Medir a tradução relocável com a máquina quente (aquecimento).
- [ ] Validar o que foi trazido do ps3recomp v0.19 (#670, #693, #635, #666,
  #659) e atualizar o patch do toolkit para a v0.19.
- [ ] As fases depois de `bay area`.
- [ ] `RES_LEVEL=<n>`: confirmar se o EBOOT aceita `-level`.
- [ ] Descobrir por que o front end deixa o port 0 desabilitado e trocar o
  hook pela correção da causa real.
- [ ] 30 fps constantes no jogo todo, sem travamentos. Em 30/10/2026 a
  gameplay normal fica em 28–30 fps (eram 14,5) e os quadros de 100 ms ou
  mais caíram de 27 para uns 3 por minuto; falta o combate pesado.
- [ ] Depois, 60 fps: trazer os patches da comunidade de destrave de quadros
  e campo de visão do BCUS98120 para o `port/patches.cpp` e rodar o relógio de
  vblank a 120 Hz com `g_res_vblank_hz` (que já existe no `port/patches.cpp`),
  como o port do Resistance: Fall of Man faz com `RES_UNLOCK_FPS=1`.
- [x] O launcher de desktop (`launcher/`, adaptado do port do Resistance: Fall
  of Man), com instalação em um clique e descriptografia sem o RPCS3.
- [ ] Testar a instalação em um clique do começo ao fim num Mac limpo; um
  build de release do launcher.
- [ ] RecompLauncher: o perfil (`recomplauncher.json`), o `--prepare` sem
  janela do launcher e o `scripts/package-recomplauncher.sh` já estão aqui;
  falta uma release com esse zip, um prepare completo num Mac limpo e a
  listagem no recomplauncher.com (README do launcher, "RecompLauncher").
  Online pelo RecompLauncher (12/11/2026): a instalação compila a versão
  01.60 depois da campanha (sem nunca fazê-la falhar), um bloco Online roda
  o `--play-online` sem janela nos 30 fps do próprio jogo, o online ID é
  definido na janela do launcher (o perfil não tem configuração de texto),
  códigos de saída 47 a 51; testado só com entradas falsas, falta uma
  instalação de verdade e um início online pelo RecompLauncher.
- [ ] Mandar as correções genéricas do toolkit para o upstream (tabelas de
  workloads por instância, a correção do despertar perdido, a aposentadoria
  de overlays, `ESRCH`, NV0039, a releitura do `put` no FIFO, a amostragem
  de recorte).

## Mais adiante

- [ ] Multiplayer online (competitivo e o cooperativo de 8 jogadores). A Sony
  fechou os servidores do Resistance em abril de 2014. Projetos da comunidade
  estão trazendo-os de volta para consoles reais e para o RPCS3: há relatos
  de que o PS Rewired roda o cooperativo e as partidas ranqueadas do
  Resistance 2 (acessado trocando o DNS do console), e o VGFury mostrou um
  primeiro alpha de outra restauração em setembro de 2026. O port do
  Resistance: Fall of Man já tem a base no patch do toolkit dele (11/11/2026):
  sockets de verdade (`PS3_NET_ONLINE=1`), consultas por um DNS escolhido
  (`PS3_NET_DNS`), login e ticket da PSN feitos localmente
  (`PS3_NP_SIGNIN=1`), e com isso aquele jogo entra no PS Rewired. Neste
  port essas mudanças estão no patch do toolkit desde 11/11/2026, todas
  desligadas por padrão (o jogo offline não muda; os testes unitários delas
  passam aqui). O guia do PS Rewired para o RPCS3 lista o Resistance 2 com o
  DNS `67.222.156.250` ou com a troca de host
  `resistance2.ps3.online.scea.com=67.222.156.250`; esse é o nome de servidor
  que está no executável deste jogo, e `PS3_NET_DNS` / `PS3_NET_HOSTS` cobrem
  os dois jeitos. Primeira tentativa (11/11/2026): com essas chaves a versão
  do disco (01.00) abre, se considera online, recebe o ticket e chega ao menu
  Cooperative, mas a lógica de login do próprio jogo responde "You've been
  signed out of the PlayStation®Network" e não chega a falar com o servidor.
  Um erro do port achado no caminho foi corrigido (o jogo reserva a porta UDP
  1900 para procurar um roteador UPnP; com outro programa nessa porta a
  reserva falhava). A lista de jogo online do RPCS3 diz que este jogo
  funciona no PS Rewired com a atualização 01.60, e o servidor do Resistance:
  Fall of Man só aceitou a última atualização daquele jogo. Então o online
  agora roda a partir de um segundo build, o da atualização 01.60, mantendo
  a 01.00 para a campanha (12/11/2026): o `tools/build_online.sh` baixa os
  dois pacotes oficiais da atualização, confere e extrai os dois e compila o
  executável da 01.60 (decifrado pelo `tools/decrypt_self.py` sem o RPCS3
  desde 12/11/2026: licença NPDRM gratuita; o resultado tem o SHA-256
  conhecido), e o **Jogar online** do launcher o roda numa pasta
  própria (pastas de HDD próprias: um `dev_hdd1` vazio, já que o cache da
  instalação da 01.00 o deixa em laço no boot, e os patches de dados da
  atualização, que ele só monta quando ouve que a instalação no HD existe)
  nos 30 fps do próprio jogo, como as regras do servidor exigem. Em
  12/11/2026 o build 01.60 roda, faz login e chega ao contrato de uso do PS
  Rewired. Ainda não testados: o lobby, a lista de partidas, uma partida. A
  campanha vem primeiro.
