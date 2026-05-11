# Criar Dev Sessions a partir de Sprint

Crie os arquivos de dev-session para um sprint existente. Cada session é um arquivo autocontido
com instruções suficientes para ser executada de forma independente (dados os pré-requisitos).

Os arquivos devem ser salvos em `docs/dev-sessions-sprint-{N}/session-{NN}-{slug}.md`.

## Instruções

1. Leia o sprint indicado em `docs/sprint-{N}.md` completamente.
2. Leia pelo menos 2 diretórios de dev-sessions existentes para calibrar o formato.
3. Leia os arquivos-fonte referenciados no sprint para entender o estado atual do código.
4. Leia `CLAUDE.md` para contexto da arquitetura.
5. Decomponha os entregáveis do sprint em sessions incrementais — cada session produz um artefato verificável.
6. Crie o diretório `docs/sprints/dev-sessions-sprint-{N}/` e todos os arquivos de session.

## Template de Dev Session

```markdown
# Session {NN} — {Título descritivo da tarefa discreta}

**Pré-reqs:** {Session anterior ou Sprint} verde
**Saída:** {artefato concreto + comando de verificação que deve passar}

---

## Objetivo

{2-3 frases descrevendo o que esta session entrega e por quê.
Incluir referência ao sprint: "Fonte: sprint-{N}.md §{seção}"}

---

## [Contexto] (opcional)

{Se a session exige decisão de design ou tem nuance técnica não-óbvia,
explicar aqui. Se o objetivo é autoexplicativo, omitir esta seção.}

---

## Passos

### 1. `path/relativo/ao/arquivo.rs` — {descrição curta da mudança}

{Explicação do que mudar, com ANTES/DEPOIS quando modificando código existente.}

```rust
// código completo ou diff relevante
```

### 2. {próximo passo}

{Cada passo é um arquivo ou mudança atômica. Usar ### numerado.
Incluir design notes em texto quando a decisão não é óbvia.}

---

## Verificação

```bash
{comandos exatos — cargo build/test/clippy}
```

**Esperado:** {descrição do resultado correto}

{Se não há testes unitários nesta session, explicar por quê e onde a cobertura vem
(smoke test manual, integration test em session posterior, etc.)}

---

## [Notas sobre X] (opcional)

{Seção adicional para edge cases, testabilidade, alternativas futuras.
Só incluir quando agrega informação que o implementador precisaria.}

---

## DoD

- [ ] {item verificável — arquivo existe, teste passa, campo presente}
- [ ] {item verificável}
- [ ] `cargo {comando}` verde
```

## Template de Session Gate Final

A última session de toda sprint é sempre o gate final:

```markdown
# Session {NN} — Gate Final: Todos os Gates Passam

**Pré-reqs:** Sessions 01-{NN-1} completas
**Saída:** Todos gates passam; sprint completa

---

## Objetivo

Verificação final. Rodar todos os gates, corrigir qualquer regressão,
confirmar que o workspace inteiro continua verde.

---

## Gates

```bash
cargo build --all-features
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
{make bench — se sprint tem impacto de performance}
```

---

## Checklist Final

### Estrutura

- [ ] {verificações de arquivos/diretórios criados}

### Funcionalidade

- [ ] {comportamentos observáveis que devem funcionar}

### Qualidade

- [ ] `cargo test` — {N}+ testes verdes
- [ ] `cargo clippy` — zero warnings
- [ ] `cargo fmt` — formatado
- [ ] {restrições específicas: sem unwrap em produção, etc.}

### Não introduzido (deliberadamente)

- [ ] {features adiadas para sprints futuros}

---

## Se Algum Gate Falhar

{Lista numerada com diagnóstico provável e ação corretiva por tipo de falha:
build error, test failure, clippy warning, bench regression, etc.}

---

## DoD (Sprint {N} completa)

- [ ] Todos os gates passam
- [ ] Smoke test manual funcional
- [ ] {critério de shippability}
```

## Regras

- Escreva em português brasileiro
- Cada session é **autocontida** — alguém pode abrir o arquivo e executar sem ler as outras sessions (dados os pré-reqs)
- Sessions produzem artefatos incrementais verificáveis — `cargo build` ou `cargo test` verde após cada uma
- Código é **real, não pseudo-código** — snippets completos com imports, types, assinaturas
- Usar `**ANTES:**` / `**DEPOIS:**` ao modificar código existente
- Numerar sessions com zero-pad: `01`, `02`, ..., `09`, `10`
- Slug do filename é curto e descritivo: `session-03-bridge.md`, não `session-03-implement-the-bridge-module.md`
- Sem emojis
- DoD usa `- [ ]` checkbox format
- Verificação inclui comandos bash exatos
- Se uma session não tem testes unitários, explicar onde a cobertura vem (integration test, smoke test, session posterior)
- Gate final é sempre a última session
- Session gate inclui seção "Se Algum Gate Falhar" com diagnósticos
- Referência ao sprint: `Fonte: sprint-{N}.md §{seção}`
- Design notes inline nos passos — não em seção separada

$ARGUMENTS
