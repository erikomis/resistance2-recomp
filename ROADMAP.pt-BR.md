# Roadmap

🌐 [English](ROADMAP.md) · **Português (BR)**

Estado do port nativo do Resistance 2 (`BCUS98120`) para macOS/arm64 e o que
vem a seguir. Mesma organização do roadmap do Resistance: Fall of Man.
`[x]` = feito e verificado; `[ ]` = a fazer; "(em andamento)" = sendo feito.

## Resumo (29/10/2026)

- **Gameplay controlável na primeira fase:** boot → menus → New Game →
  vídeos de abertura a 30 fps → cinemática na engine → gameplay em primeira
  pessoa com HUD em `iceland`; o jogador anda.
- **Uns 18 fps jogando, com quedas:** código SPU relocado ainda
  interpretado, compilação síncrona de shaders.
- **Imagem:** a metade esquerda da tela composta errada em menus e
  cinemáticas; jogando, a imagem ocupa ~1152×654 com uma borda clara à
  direita e embaixo.

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

## Próximos passos

- [ ] (em andamento) Desempenho na fase: traduzir o código SPU que a fase
  copia para endereços variáveis do LS (uma tradução relocável, como os
  módulos relocáveis do inFAMOUS 2) e tirar a compilação de shaders do
  quadro.
- [ ] A metade esquerda da tela em menus e cinemáticas (buffer de 2560 de
  largura / pós-processamento).
- [ ] A imagem jogando em ~1152×654 com borda clara (viewport / escala do
  buffer de exibição).
- [ ] As travadas intermitentes e o encerramento que precisa de um kill.
- [ ] Jogar a primeira fase inteira à mão; saves, áudio jogando.
- [ ] `RES_LEVEL=<n>`: confirmar se o EBOOT aceita `-level`.
- [ ] Descobrir por que o front end deixa o port 0 desabilitado e trocar o
  hook pela correção da causa real.
- [ ] Trazer os patches da comunidade de 60 fps e campo de visão do
  BCUS98120 para o `port/patches.cpp`, como no port do Resistance: Fall of Man.
- [ ] O launcher de desktop, quando o jogo estiver jogável.
- [ ] Mandar as correções genéricas do toolkit para o upstream (tabelas de
  workloads por instância, a correção do despertar perdido, a aposentadoria
  de overlays, `ESRCH`).
