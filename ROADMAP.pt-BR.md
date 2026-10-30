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

## Próximos passos

- [ ] (em andamento) Desempenho na fase: traduzir o código SPU que a fase
  copia para endereços variáveis do LS (uma tradução relocável, como os
  módulos relocáveis do inFAMOUS 2) e tirar a compilação de shaders do
  quadro.
- [ ] O encerramento que precisa de um kill.
- [ ] Jogar a primeira fase inteira à mão, com combate; saves, áudio
  jogando; as fases seguintes.
- [ ] `RES_LEVEL=<n>`: confirmar se o EBOOT aceita `-level`.
- [ ] Descobrir por que o front end deixa o port 0 desabilitado e trocar o
  hook pela correção da causa real.
- [ ] 30 fps constantes no jogo todo, sem travamentos. Em 30/10/2026 a
  gameplay da primeira fase roda a cerca de 28 fps de média e 30 de mediana
  (eram 14,5), e os quadros de 100 ms ou mais caíram de 27 para 4 por minuto:
  mapas de sombra e reduções de profundidade ficam na GPU
  (`PS3RECOMP_ENG_ZETA16_GPU`, `PS3RECOMP_ENG_BLIT_GPU`), leitura de vértices
  por atributo (upstream #458, `PS3RECOMP_ENG_FETCH_MANY`), cache de vértices
  que não recomeça mais (`PS3RECOMP_ENG_VCACHE_KEEP2`), decodificação de
  texturas swizzled mais rápida (upstream #578) e traduções de shader em cache
  entre sessões e builds, compiladas em segundo plano (`tools/play.sh`).
  Falta: esperas da GPU em cópias de cor, hash do cache de vértices,
  decodificação do FIFO, as superfícies em cores neon ao mirar e em áreas
  mais adiante.
- [ ] Depois, 60 fps: trazer os patches da comunidade de destrave de quadros
  e campo de visão do BCUS98120 para o `port/patches.cpp` e rodar o relógio de
  vblank a 120 Hz com `g_res_vblank_hz` (que já existe no `port/patches.cpp`),
  como o port do Resistance: Fall of Man faz com `RES_UNLOCK_FPS=1`.
- [ ] O launcher de desktop, quando o jogo estiver jogável.
- [ ] Mandar as correções genéricas do toolkit para o upstream (tabelas de
  workloads por instância, a correção do despertar perdido, a aposentadoria
  de overlays, `ESRCH`, NV0039, a releitura do `put` no FIFO, a amostragem
  de recorte).
