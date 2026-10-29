# Roadmap

🌐 [English](ROADMAP.md) · **Português (BR)**

Estado do port nativo do Resistance 2 (`BCUS98120`) para macOS/arm64 e o que
vem a seguir. Mesma organização do roadmap do Resistance: Fall of Man.
`[x]` = feito e verificado; `[ ]` = a fazer; "(em andamento)" = sendo feito.

## Resumo (29/10/2026)

- **Inicia até a primeira fase:** boot, aviso de auto-save, vídeos de
  abertura, menus, New Game e a primeira fase desenhada pela engine.
- **Vídeos a 30 fps:** o decodificador Bink do raw SPU é traduzido (ele
  rodava no interpretador de SPU a ~3 fps).
- **Ainda não dá para jogar:** uns 5 fps dentro da fase, uma borda branca em
  volta da imagem e problemas intermitentes.

## Feito

- [x] O toolkit: o ps3recomp v0.17.0 do upstream mais o patch de toolkit do
  Resistance: Fall of Man, num patch só com as mudanças do Resistance 2
  (`patches/ps3recomp-a6d76c8-resistance2.patch`).
- [x] EBOOT descriptografado, analisado e traduzido (umas 41.500 funções, 24
  unidades de tradução); compila e linka no macOS/arm64 com Metal.
- [x] Os ELFs de SPU embutidos traduzidos pelo build; o carregador do raw SPU
  (`igInitRawSpu`) recortado do EBOOT e traduzido junto.
- [x] O módulo de política SPU Wws_Job (bytes crus em `0x00CF9680` do EBOOT)
  traduzido e registrado pela assinatura.
- [x] Controle no aviso de auto-save: o hook do port 0 do front end
  (`tools/apply_r2_frontend_patch.py`, `RES_FRONTEND_PORT0`).
- [x] `cellHttpClientPollConnections` responde "indisponível" em vez de um
  sucesso falso, e a thread HTTP do jogo espera em vez de girar à toa.
- [x] O decodificador Bink do raw SPU traduzido: vídeos a 30 fps.
- [x] Despertar perdido no `WAIT_SIGNAL` do SPURS (travamento ao carregar a
  fase) corrigido.
- [x] Travamento do "New Game": `ESRCH` para uma thread que já acabou.
- [x] A primeira fase é desenhada pela engine.

## Próximos passos

- [ ] (em andamento) Traduzir os 29 módulos de job WWS
  (`tools/regen_r2_jobmods.sh`), conferir os que têm instruções não
  suportadas e medir a taxa de quadros da fase (uns 5 fps hoje).
- [ ] A borda branca em volta da imagem na fase (viewport / scissor /
  tamanho da superfície).
- [ ] As travadas intermitentes e o encerramento que precisa de um kill.
- [ ] Jogar a primeira fase inteira com controle; saves, áudio jogando.
- [ ] `RES_LEVEL=<n>`: confirmar se o EBOOT aceita `-level`.
- [ ] Descobrir por que o front end deixa o port 0 desabilitado e trocar o
  hook pela correção da causa real.
- [ ] Trazer os patches da comunidade de 60 fps e campo de visão do
  BCUS98120 para o `port/patches.cpp`, como no port do Resistance: Fall of Man.
- [ ] O launcher de desktop, quando o jogo estiver jogável.
- [ ] Mandar as correções genéricas do toolkit para o upstream.
