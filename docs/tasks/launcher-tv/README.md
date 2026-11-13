# Backlog — launcher-tv (Sofá)

| Campo | Valor |
|---|---|
| Spec | `docs/specs/launcher-tv.md` (CA-01 a CA-76, RN-01 a RN-30, decisões D-01 a D-28) |
| Arquitetura | `docs/arch/launcher-tv.md` (rev. 2) + ADRs 0001 a 0006 (0003, 0005 e 0006 `proposed` até os spikes) |
| Segurança | `docs/security/launcher-tv.md` rev. 2 (SEC-01 a SEC-32, AF-01 a AF-10; emendas em SEC-08, SEC-10, SEC-21, SEC-22, SEC-25, SEC-28, SEC-29; DR-03 decidido: desinstalar apaga só os dados do usuário interativo) |
| Marca | `docs/brand/sofa/` (concluída) |
| Branch | `feature/launcher-tv-20261113` (base `main`) |
| Painel | `docs/tasks/launcher-tv/painel.html` |
| Data | 2026-11-13 |

## Agentes e arquivos

| Agente | Arquivo | Tarefas | Papel nesta feature |
|---|---|---|---|
| dev-backend | `dev-backend.md` | 43 (BE-01 a BE-43) | Spikes técnicos, contrato (comandos Tauri + eventos), núcleo Rust, persistência JSON, instalador e desinstalador |
| dev-frontend | `dev-frontend.md` | 14 (FE-01 a FE-14) | UI React no WebView2 |
| designer-ux | `designer-ux.md` | 9 (DES-01 a DES-09) | Mockups HTML sobre os tokens da marca, microcopy, aprovação com o usuário, revisão da UI |
| qa | `qa.md` | 17 (QA-01 a QA-17) | Plano, E2E, testes em VM e com apps sintéticos, segurança dinâmica, roteiro único do usuário, relatório |
| seguranca | `seguranca.md` | 10 (SG-01 a SG-10) | Revisão do contrato, dos spikes SP-01 e SP-07 e dos ADRs revisados, verificação do código, decisão final dos degraus 2 e 3 |
| arquiteto | `arquiteto.md` | 5 (ARQ-01 a ARQ-05) | Fechar ADRs com os spikes técnicos, revisão final |

Total: 98 tarefas. A rev. 2 do threat model acrescentou BE-41, BE-42, SG-09 e QA-15; o replanejamento sem o usuário nos spikes acrescentou QA-16, QA-17, SG-10 e BE-43.

**Decisões do usuário (painel):** D-29 lista de jogos enviada depois; D-30 anti-cheat testado em modo offline, no fim; D-31 conta padrão + outro admin testados no fim; D-32 PC da sala é Windows 10 (alvo principal), Windows 11 secundário; D-33 hardware ausente = "não verificado"; D-34 CI = `scripts/ci.ps1` local + pasta de releases; D-35 o usuário só testa no fim, sem spikes com ele. Não participam: modelador-bd (sem banco; a persistência JSON do ADR-0004 está em BE-11), designer-marca (concluído) e produto (só se o escopo mudar).

Status: todas começam em `a fazer`. Valores aceitos: `a fazer`, `em andamento`, `feito`, `bloqueado: <motivo>`.

## Ordem de execução

| Fase | Tarefas | Observação |
|---|---|---|
| 0. Já pode começar | BE-01 a BE-05 (spikes técnicos), BE-06, BE-07, DES-01, QA-01 | Spikes na máquina de dev e em VM, com apps sintéticos; não dependem do usuário |
| 1. Decisões e base | SG-01, SG-02, SG-03, ARQ-01 a ARQ-04; BE-08 a BE-13, BE-19, BE-22, BE-23, BE-31; FE-01; DES-02 a DES-06 | Contrato revisado (SG-01) antes de BE-08 e BE-10; ADRs fechados antes dos módulos que dependem deles |
| 2. Módulos do núcleo e aprovação do design | BE-14 a BE-18, BE-24 a BE-28, BE-32 a BE-36; SG-09; DES-07, DES-08; FE-02 | DES-08 é a aprovação dos mockups pelo usuário: nenhuma tela do front começa antes. SG-09 confere os ADRs 0003 e 0006 revisados |
| 3. Fluxos e telas | BE-20, BE-21, BE-29, BE-30, BE-37 a BE-42; FE-03 a FE-14 | Fatias verticais: cada tela entra quando o comando do núcleo correspondente fica pronto. Desinstalador (BE-38, BE-41, BE-42) só depois de SG-09 |
| 4. Verificação sem o usuário | QA-02 a QA-12, QA-15, SG-04 a SG-07, DES-09 | QA (automatizado, apps sintéticos, VMs Windows 10 e 11) e seguranca verificam juntos |
| 5. Roteiro do usuário | QA-16 (prepara exe + passo a passo + coletor de logs), QA-17 (usuário executa no PC da sala; QA analisa) | Única participação do usuário em teste, além da aprovação dos mockups (DES-08) |
| 6. Fechamento | SG-10, BE-43, QA-13, SG-08, ARQ-05; depois QA-14 (2 semanas de uso, não bloqueia) | SG-10 decide os degraus 2 e 3 com os logs reais; BE-43 aplica |

## Caminho crítico

```text
BE-01 (SP-01 técnico) → SG-02 → ARQ-01 → BE-14 / BE-18 → BE-20 → BE-21 → FE-09 → QA-06 → QA-16 → QA-17 (usuário) → SG-10 → BE-43 → ARQ-05
```

SG-10 também libera SG-08 e QA-17 libera QA-13; os dois entram em ARQ-05 em paralelo com BE-43.

Entram nesse caminho por BE-20, FE-09 e QA-16:

- **Contrato**: BE-07 → SG-01 → BE-10 → BE-14.
- **Biblioteca**: BE-03 (SP-08) → ARQ-04 → BE-25/26/27 → BE-29 → BE-20.
- **Design**: DES-01 → DES-02..06 → DES-07 → DES-08 (aprovação do usuário) → FE-03 → FE-05 → FE-08 → FE-09.
- **Instalador**: BE-04 → SG-03 → ARQ-03 → BE-37 → BE-39 → QA-16 (o roteiro usa o instalador de release).

Os spikes não esperam mais pelo usuário. O risco de atraso passa para a aprovação dos mockups, para a lista de jogos (D-29, necessária só no QA-16) e para a disponibilidade do usuário em executar o roteiro (QA-17).

**Em paralelo**: os 5 spikes entre si; BE-06/BE-07/DES-01/QA-01 junto com os spikes; domínio puro (BE-13, BE-15, BE-22, BE-23, BE-31) enquanto os ADRs fecham; scanners Steam, Epic e GOG (BE-25, BE-26, BE-27) entre si; volume (BE-31, BE-32), energia (BE-33), monitor (BE-34) e instalação (BE-35 a BE-39) em trilhas separadas; telas FE-06, FE-07, FE-10, FE-11 e FE-12 depois de FE-03/FE-04.

## Tarefas que dependem do usuário

| Tarefa | O que precisa | Dúvida |
|---|---|---|
| DES-08 | Aprovar os mockups | — |
| QA-16 | Enviar a lista de jogos (nome, loja, modo de tela) antes de o roteiro ser fechado | D-29 |
| QA-17 | Executar o roteiro único no PC da sala (Windows 10): instalar, boot, controle, jogos em cada modo, anti-cheat em modo offline, energia, TV, conta padrão + outro admin, desinstalar; rodar o coletor e devolver o zip | D-30, D-31, D-32, D-33, D-35 |
| QA-14 | 2 semanas de uso real (não bloqueia a revisão final) | — |

As dúvidas do painel começam em D-29 porque a spec já usa D-01 a D-28 para as decisões registradas.

## Grafo de dependências

```mermaid
flowchart LR
  subgraph Spikes
    BE01[BE-01 SP-01]
    BE02[BE-02 SP-02]
    BE03[BE-03 SP-08]
    BE04[BE-04 SP-07]
    BE05[BE-05 SP-06]
  end
  subgraph Decisoes[Decisões]
    SG01[SG-01]
    SG02[SG-02]
    SG03[SG-03]
    ARQ01[ARQ-01]
    ARQ02[ARQ-02]
    ARQ03[ARQ-03]
    ARQ04[ARQ-04]
  end

  BE01 --> SG02
  BE01 & SG02 --> ARQ01
  BE02 --> ARQ02
  BE04 --> SG03
  BE04 & SG03 --> ARQ03
  BE03 & BE05 --> ARQ04
  BE07[BE-07 contrato] --> SG01

  BE06[BE-06] & SG01 --> BE08[BE-08]
  BE08 --> BE09[BE-09]
  BE08 & SG01 --> BE10[BE-10]
  BE06 --> BE11[BE-11] & BE12[BE-12] & BE13[BE-13] & BE19[BE-19] & BE22[BE-22] & BE23[BE-23] & BE31[BE-31]
  BE13 & BE10 & ARQ01 --> BE14[BE-14]
  BE06 & ARQ02 --> BE15[BE-15]
  BE15 & BE23 --> BE16[BE-16]
  BE15 & BE23 & ARQ02 --> BE17[BE-17]
  BE06 & ARQ01 --> BE18[BE-18]
  BE14 & BE16 & BE17 & BE18 & BE29 & ARQ04 --> BE20[BE-20]
  BE20 --> BE21[BE-21]
  BE23 & BE10 --> BE24[BE-24]
  BE23 & ARQ04 --> BE25[BE-25] & BE26[BE-26] & BE27[BE-27]
  BE22 & BE11 --> BE28[BE-28]
  BE10 & BE11 & BE22 & BE25 & BE26 & BE27 & BE28 --> BE29[BE-29]
  BE24 & BE29 --> BE30[BE-30]
  BE31 & BE10 --> BE32[BE-32]
  BE10 --> BE33[BE-33]
  BE10 & BE11 & BE19 --> BE34[BE-34]
  BE08 & ARQ03 --> BE35[BE-35]
  BE10 & BE11 & ARQ03 --> BE36[BE-36]
  BE09 & ARQ03 --> BE37[BE-37]
  ARQ01 & ARQ03 --> SG09[SG-09]
  BE11 & BE35 & SG09 --> BE41[BE-41]
  BE37 & SG09 --> BE42[BE-42]
  BE37 & BE41 & SG09 --> BE38[BE-38]
  BE37 & BE12 --> BE39[BE-39]
  BE29 & BE34 & BE36 & FE05 --> BE40[BE-40]

  DES01[DES-01] --> DES02[DES-02] & DES03[DES-03] & DES04[DES-04] & DES05[DES-05] & DES06[DES-06]
  DES02 & DES03 & DES04 & DES05 & DES06 & BE07 --> DES07[DES-07]
  DES07 --> DES08[DES-08 aprovação]

  BE06 --> FE01[FE-01]
  BE10 & DES07 --> FE02[FE-02]
  FE01 & FE02 & DES08 --> FE03[FE-03]
  FE01 & DES08 --> FE04[FE-04]
  FE03 & FE04 & BE29 --> FE05[FE-05]
  FE03 & FE04 & BE14 --> FE06[FE-06]
  FE05 --> FE07[FE-07]
  FE05 & BE20 --> FE08[FE-08]
  FE08 & BE21 --> FE09[FE-09]
  FE03 & FE04 & BE32 & BE33 & BE35 --> FE10[FE-10]
  FE05 & BE32 --> FE11[FE-11]
  FE03 & FE04 & BE29 & BE34 & BE36 --> FE12[FE-12]
  FE12 & BE30 --> FE13[FE-13]
  FE03 & BE19 --> FE14[FE-14]

  QA01[QA-01] & FE10 & BE09 --> QA02[QA-02]
  QA01 & FE06 & BE14 --> QA03[QA-03]
  QA01 & FE07 & FE12 & BE29 --> QA04[QA-04]
  QA01 & FE13 & BE30 --> QA05[QA-05]
  QA01 & FE09 & BE21 --> QA06[QA-06]
  QA01 & FE10 & FE11 --> QA07[QA-07]
  QA01 & FE10 & FE14 & BE33 --> QA08[QA-08]
  QA01 & FE12 & BE34 & BE40 --> QA09[QA-09]
  QA01 & BE35 & BE36 & BE38 & BE39 & BE40 --> QA10[QA-10]
  QA01 & BE21 & BE29 & BE30 & FE05 --> QA11[QA-11]
  QA01 & BE11 & BE29 --> QA12[QA-12]
  QA01 & BE38 & BE41 & BE42 --> QA15[QA-15]
  QA02 & QA03 & QA04 & QA05 & QA06 & QA07 & QA08 & QA09 & QA10 & QA11 & QA12 & QA15 & BE39 & BE40 --> QA16[QA-16 roteiro]
  QA16 --> QA17[QA-17 usuário]
  QA17 & SG06 --> SG10[SG-10]
  SG10 --> BE43[BE-43]
  QA02 & QA03 & QA04 & QA05 & QA06 & QA07 & QA08 & QA09 & QA10 & QA11 & QA12 & QA15 & QA17 --> QA13[QA-13]
  QA13 --> QA14[QA-14]

  BE08 & BE09 & BE10 & BE28 & BE29 & FE02 & FE04 & FE05 --> SG04[SG-04]
  BE16 & BE23 & BE24 & BE25 & BE26 & BE27 & BE30 --> SG05[SG-05]
  BE14 & BE17 & BE18 & BE19 & BE21 --> SG06[SG-06]
  BE11 & BE12 & BE35 & BE36 & BE37 & BE38 & BE39 & BE41 & BE42 & QA15 --> SG07[SG-07]
  BE32 & BE33 & SG04 & SG05 & SG06 & SG07 & SG10 & QA11 --> SG08[SG-08]

  FE05 & FE07 & FE08 & FE09 & FE10 & FE11 & FE12 & FE13 --> DES09[DES-09]
  QA13 & SG08 & DES09 & BE39 & BE40 & BE43 --> ARQ05[ARQ-05 revisão final]
```

## Matriz de cobertura

"Implementa" inclui spikes e design que alimentam a entrega. "Verifica" é a tarefa de teste (QA) ou de verificação (SG/DES) que confirma o critério.

### Critérios de aceite

| CA | Implementa | Verifica |
|---|---|---|
| CA-01 | BE-34, BE-36, BE-40 | QA-09, QA-10 |
| CA-02 | BE-35 | QA-10 |
| CA-03 | BE-08, FE-03, DES-01 | QA-09 |
| CA-04 | BE-36, FE-12, DES-06 | QA-10 |
| CA-05 | BE-35, FE-10, DES-05 | QA-08 |
| CA-06 | BE-13, BE-14, FE-03, DES-02 | QA-03 |
| CA-07 | BE-13, FE-03 | QA-03 |
| CA-08 | BE-13 | QA-03 |
| CA-09 | FE-03, DES-08 | QA-02, QA-03 |
| CA-10 | FE-03, DES-08 | QA-02, QA-03 |
| CA-11 | BE-13, BE-14 | QA-03 |
| CA-12 | BE-13, BE-14, BE-20 | QA-03, QA-06 |
| CA-13 | BE-14, FE-06, DES-02, DES-07 | QA-03 |
| CA-14 | BE-14, FE-06, DES-02 | QA-03 |
| CA-15 | BE-14, FE-06, DES-02 | QA-03 |
| CA-16 | BE-03, BE-25, BE-28, BE-29, FE-12 | QA-04 |
| CA-17 | BE-25, BE-29 | QA-04 |
| CA-18 | BE-25, BE-29 | QA-04 |
| CA-19 | BE-24, BE-28, BE-30, FE-13, DES-06 | QA-05 |
| CA-20 | BE-22, BE-30, FE-13, DES-06, DES-07 | QA-05 |
| CA-21 | BE-22, BE-29, FE-07, DES-03 | QA-04 |
| CA-22 | BE-22, BE-29, FE-05, DES-02 | QA-04 |
| CA-23 | FE-05, DES-02 | QA-04 |
| CA-24 | BE-28, FE-04, DES-01, DES-02 | QA-04 |
| CA-25 | BE-11, BE-22, BE-29 | QA-12 |
| CA-26 | BE-16, BE-20, FE-08, DES-04 | QA-06 |
| CA-27 | BE-16, BE-18, BE-20 | QA-06 |
| CA-28 | BE-20, BE-30, FE-08, DES-04, DES-07 | QA-05, QA-06 |
| CA-29 | BE-02, BE-15, BE-20, FE-08, DES-04 | QA-06 |
| CA-30 | BE-15, BE-17, BE-18, BE-21 | QA-06 |
| CA-31 | BE-15, BE-20, FE-08, DES-04 | QA-06 |
| CA-32 | BE-01, BE-18, BE-21, BE-43, FE-09, DES-04 | QA-06, QA-17, SG-10 |
| CA-33 | BE-13 | QA-06 |
| CA-34 | BE-18, BE-21, FE-09, DES-04 | QA-06 |
| CA-35 | BE-15, BE-21, FE-09, DES-04 | QA-06 |
| CA-36 | BE-21, FE-05, FE-09, DES-02, DES-04 | QA-06 |
| CA-37 | FE-09, FE-10, DES-05 | QA-06 |
| CA-38 | BE-31, BE-32, FE-10, DES-05 | QA-07 |
| CA-39 | BE-31, FE-10, DES-05 | QA-07 |
| CA-40 | BE-31, FE-10, FE-11 | QA-07 |
| CA-41 | BE-32, FE-10 | QA-07 |
| CA-42 | BE-32, FE-10, DES-05 | QA-07 |
| CA-43 | BE-33, FE-04, FE-10, DES-05 | QA-02, QA-08 |
| CA-44 | BE-33, FE-10 | QA-08 |
| CA-45 | BE-33, FE-10 | QA-08, QA-10 |
| CA-46 | BE-19, BE-33, FE-10, FE-14 | QA-08 |
| CA-47 | BE-33, FE-10, DES-05 | QA-08 |
| CA-48 | FE-10, DES-05 | QA-08 |
| CA-49 | BE-33 | QA-08 |
| CA-50 | BE-37, DES-07 | QA-10 |
| CA-51 | BE-04, BE-36, BE-37 | QA-10 |
| CA-52 | BE-38, BE-42 | QA-10, QA-15 |
| CA-53 | BE-37 | QA-10 |
| CA-54 | DES-01, FE-03, FE-04 | QA-09, DES-09 |
| CA-55 | DES-01, FE-03, FE-04 | QA-03, QA-09, DES-09 |
| CA-56 | BE-26, BE-28, BE-29 | QA-04 |
| CA-57 | BE-27, BE-28, BE-29 | QA-04 |
| CA-58 | BE-26, BE-27, BE-29 | QA-04 |
| CA-59 | BE-26, BE-27, BE-29 | QA-04 |
| CA-60 | BE-22, FE-04, FE-05, DES-02 | QA-04, DES-09 |
| CA-61 | BE-29, FE-07, DES-03 | QA-04 |
| CA-62 | BE-22, BE-29, FE-07, DES-03 | QA-04 |
| CA-63 | BE-01, BE-18, BE-21, BE-43, FE-09, DES-04 | QA-06, QA-17, SG-10 |
| CA-64 | BE-13, BE-31, FE-11, DES-02 | QA-07 |
| CA-65 | BE-13, FE-11 | QA-07 |
| CA-66 | BE-19, BE-34, FE-12, DES-06 | QA-09 |
| CA-67 | BE-19, BE-34, FE-12 | QA-09 |
| CA-68 | BE-38, BE-41 | QA-10, QA-15 |
| CA-69 | BE-22, BE-25 | QA-04 |
| CA-70 | BE-29, FE-12, DES-06 | QA-04 |
| CA-71 | BE-23, BE-24, BE-30, FE-08, FE-13, DES-04, DES-06 | QA-05 |
| CA-72 | BE-30, FE-13, DES-06 | QA-05 |
| CA-73 | BE-16, BE-20, FE-08, DES-04 | QA-06 |
| CA-74 | BE-09, BE-36 | QA-10 |
| CA-75 | BE-11, BE-20, BE-22 | QA-06, QA-12 |
| CA-76 | BE-39 | QA-10 |

### Regras de negócio

| RN | Implementa | Verifica |
|---|---|---|
| RN-01 | BE-06, BE-12 | QA-11 |
| RN-02 | BE-22 | QA-04 |
| RN-03 | BE-22, BE-30 | QA-04 |
| RN-04 | BE-22, BE-29, FE-07, DES-03 | QA-04 |
| RN-05 | BE-20, BE-22, BE-29, FE-05, DES-02 | QA-04 |
| RN-06 | BE-13 | QA-03 |
| RN-07 | FE-03 | QA-03 |
| RN-08 | BE-13, BE-14 | QA-03 |
| RN-09 | BE-15, BE-20 | QA-06 |
| RN-10 | BE-02, BE-15, BE-17 | QA-06 |
| RN-11 | BE-13, BE-21 | QA-06 |
| RN-12 | BE-15, BE-21, FE-09 | QA-06 |
| RN-13 | BE-31, BE-32 | QA-07 |
| RN-14 | BE-33, DES-01, DES-03, DES-05, FE-04, FE-07, FE-10 | QA-02, QA-08 |
| RN-15 | BE-33 | QA-08 |
| RN-16 | BE-35 | QA-10 |
| RN-17 | BE-04, BE-36, BE-42 | QA-10 |
| RN-18 | BE-11, BE-29, BE-38, BE-41 | QA-10, QA-12, QA-15 |
| RN-19 | BE-37, DES-07 | QA-10 |
| RN-20 | BE-34 | QA-09 |
| RN-21 | BE-02, BE-05, BE-14, BE-17, BE-20, FE-14 | QA-06 |
| RN-22 | BE-22, BE-29, FE-07, DES-03 | QA-04 |
| RN-23 | BE-13, FE-11 | QA-07 |
| RN-24 | BE-01, BE-18, BE-21 | QA-06 |
| RN-25 | BE-22, FE-04 | QA-04 |
| RN-26 | BE-37 | QA-10 |
| RN-27 | BE-09, BE-16 | QA-06, QA-10 |
| RN-28 | BE-22, BE-25 | QA-04 |
| RN-29 | BE-23, BE-24, BE-30, FE-13 | QA-05 |
| RN-30 | BE-39 | QA-10 |

### Requisitos de segurança

| SEC | Implementa | Verifica |
|---|---|---|
| SEC-01 | BE-08, FE-01 | SG-04, QA-02 |
| SEC-02 | BE-09 | SG-04, QA-02 |
| SEC-03 | BE-07, BE-10, BE-29, FE-02 | SG-01, SG-04 |
| SEC-04 | BE-08 | SG-04 |
| SEC-05 | BE-07, BE-08 | SG-01, SG-04 |
| SEC-06 | BE-07, BE-10, BE-20, BE-29, FE-02 | SG-01, SG-04, QA-11 |
| SEC-07 | BE-23, BE-29, FE-01, FE-02, FE-04, FE-07 | SG-04, QA-11 |
| SEC-08 | BE-28, FE-04 | SG-04, QA-11 |
| SEC-09 | BE-23, BE-24, BE-30 | SG-05, QA-05 |
| SEC-10 | BE-16 (inclui a exceção AF-01) | SG-05, QA-05, QA-06 |
| SEC-11 | BE-16, BE-23, BE-25, BE-26 | SG-05, QA-11 |
| SEC-12 | BE-23, BE-25, BE-26, BE-27 | SG-05, QA-11 |
| SEC-13 | BE-25, BE-26, BE-27, BE-29 | SG-05, QA-11 |
| SEC-14 | BE-23, BE-27 | SG-05, QA-11 |
| SEC-15 | BE-15, BE-17, BE-21 | SG-06, QA-11 |
| SEC-16 | BE-17, BE-23, BE-25, BE-26, BE-27 | SG-05, QA-11 |
| SEC-17 | BE-15, BE-17, BE-21 | SG-06, QA-11 |
| SEC-18 | BE-11, BE-29, BE-30 | SG-07, QA-05, QA-12 |
| SEC-19 | BE-35 | SG-07, QA-10 |
| SEC-20 | BE-09, BE-37, BE-41, FE-12 | SG-07, QA-10 |
| SEC-21 | BE-36, BE-37 | SG-03, SG-07, QA-10 |
| SEC-22 | BE-36, BE-42 | SG-03, SG-07, SG-09, QA-10 |
| SEC-23 | BE-09, BE-37 | SG-03, SG-07, QA-10 |
| SEC-24 | BE-06, BE-12, BE-39, FE-01 | SG-07 |
| SEC-25 | BE-38, BE-41 | SG-03, SG-07, SG-09, QA-10, QA-15 |
| SEC-26 | BE-39 | SG-07, QA-10 |
| SEC-27 | BE-31, BE-32, BE-33, FE-10 | SG-08, QA-08 |
| SEC-28 | BE-12, BE-14, BE-18, BE-19, BE-43 | SG-02, SG-06, SG-09, SG-10, QA-17 |
| SEC-29 | BE-11, BE-12, BE-29, BE-36, BE-37, BE-38, BE-41, BE-42 | SG-03, SG-07 |
| SEC-30 | BE-06, BE-12, FE-01 | SG-08, QA-11 |
| SEC-31 | BE-38, BE-41 | SG-09, SG-07, QA-15 |
| SEC-32 | BE-12, BE-37, BE-42 | SG-09, SG-07, QA-15 |

Nenhum CA, RN ou SEC ficou sem tarefa de implementação e de verificação.

Além das tarefas listadas na coluna "Verifica", o roteiro do usuário (QA-16/QA-17) revalida no PC da sala tudo o que exige hardware real: controle (CA-06 a CA-15, RN-06, RN-08), sessão e retorno com jogos reais (CA-26 a CA-37, CA-63, CA-73, RN-10, RN-21, RN-24), energia e suspensão (CA-43 a CA-49), instalação e boot no Windows 10 (CA-01, CA-50 a CA-53, M-02), TV e monitores (CA-54, CA-55, CA-66, CA-67), conta padrão + outro admin (E-19, SEC-21) e anti-cheat (SEC-28, R-07). O que o usuário não puder executar fica "não verificado" no QA-13 (D-33).

## Observações

- **Termos**: a seção 3.1 da arquitetura cita "Remover" (opções do jogo) e "Jogos ocultos" (configurações). A spec usa "Ocultar" e "Itens ocultos" e não tem "excluir" no MVP (RN-04). DES-03, DES-06 e ARQ-05 seguem a spec.
- **Desinstalador (rev. 2 de segurança)**: o processo elevado só decide; a remoção dos dados roda como o usuário interativo via `sofa.exe --purge-user-data` (BE-41, SEC-31) e apaga só os dados desse usuário (DR-03). O autostart sai de todos os perfis (BE-42, SEC-32). O arquiteto está revisando os ADRs 0003 e 0006 em paralelo; SG-09 confere a revisão antes de BE-38, BE-41 e BE-42.
- **Spikes**: código fora do repo (ex.: `D:\GIT\STD\sofa-spikes\`) ou em branch descartável; resultado em `docs/arch/spikes/SP-xx.md`.
- **Métricas**: M-02 e M-04 entram no "Pronto quando" de BE-40, BE-21 e QA-06/QA-10; M-01 só fecha no QA-14, que não bloqueia a revisão final.
