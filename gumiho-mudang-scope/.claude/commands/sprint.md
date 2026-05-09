# Criar Sprint

Crie um documento de sprint seguindo o formato padronizado do projeto Athena.
O arquivo deve ser salvo em `docs/sprint-{N}.md` onde `{N}` é o próximo número sequencial.

## Instruções

1. Verifique o último sprint existente em `docs/` para determinar o número.
2. Leia pelo menos 2 sprints recentes para calibrar tom e profundidade.
3. Leia `CLAUDE.md` para contexto da arquitetura atual.
4. Gere o documento seguindo o template abaixo **exatamente**.

## Template de Sprint

```markdown
# Sprint {N} — {Título descritivo do trabalho principal}

**Duração estimada:** {faixa: "1-2 sessões" | "3-4 dias" | "10-12 dias"}
**Dependências:** {Sprint(s) anterior(es)} completo(s) e verde(s).
**DoD:** {lista compacta de critérios concretos separados por ; — compilação, testes, artefatos, comportamento observável}

---

## Contexto

{2-4 parágrafos explicando:
- O problema ou necessidade que motiva o sprint
- O estado atual do código (o que existe, o que falta)
- Por que agora (dependências satisfeitas, bloqueios removidos)
- Referências a código existente com paths relativos}

---

## [Seções específicas do sprint]

{Seções opcionais que variam conforme o tipo de trabalho:
- "Arquitetura Alvo" — pra refactors estruturais (diagrama ASCII)
- "Decisões de Design" — pra escolhas não-óbvias
- "TS → Rust mapping" — pra ports de TypeScript
- "Escopo de Mudanças" — pra renames/reorganizações}

---

## Entregáveis

### 1. {Título do entregável}

**Arquivo(s):** `path/relativo/ao/arquivo.rs`

**O que:** {1-2 frases descrevendo o que é construído/modificado}

**Implementação:**

{Código Rust completo ou snippets-chave com context suficiente.
Não pseudo-código — implementação real que pode ser copiada.
Incluir assinaturas de função, structs, traits.}

```rust
// código aqui
```

**Diferenças do {sistema anterior}:** {se aplicável}
- {bullet points comparando com implementação prévia}

**Testes:**
- `{test_name}` — {o que verifica}
- `{test_name}` — {o que verifica}

---

{Repetir ### para cada entregável numerado}

---

## Ordem de Implementação

| Sessão | Entregável | Risco | LOC aprox. |
|--------|-----------|-------|------------|
| 01 | {título} | {Low/Médio/Alto} | {estimativa} |
| 02 | {título} | {Low/Médio/Alto} | {estimativa} |
| ... | ... | ... | ... |
| {N} | Gate final | Low | ~0 |

{Grafo de dependências entre sessões em ASCII:}

```
01 ──► 02 ──► 03
              │
              └──► 04
```

---

## Verificação

### Gates:

```bash
cargo test -p {crate}
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

### Smoke test manual:

{Passos numerados com comportamento esperado}

---

## Riscos e Mitigações

| Risco | Mitigação |
|-------|-----------|
| {descrição concreta} | {ação concreta, não "ter cuidado"} |

---

## Invariantes

{Lista numerada de propriedades que DEVEM ser preservadas}

1. {invariante}
2. {invariante}

---

## Arquivos a modificar

| Arquivo | Mudança |
|---------|---------|
| `path/to/file.rs` | {descrição curta} |
```

## Regras

- Escreva em português brasileiro
- Use backticks para identificadores: `NomeTrait`, `método()`, `CONSTANTE`
- Use `**Arquivo:**` em bold antes de code blocks
- Tabelas com `|` pipes alinhados
- Checklists com `- [ ]`
- Código Rust real, não pseudo-código
- Sem emojis
- Seções opcionais só quando relevantes — não force "Decisões de Design" se não há decisões não-óbvias
- Gate final sempre como última sessão
- Riscos com mitigações concretas (nunca "tomar cuidado" ou "monitorar")
- Invariantes são propriedades do sistema, não tarefas

$ARGUMENTS
