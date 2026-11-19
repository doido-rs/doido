# PROMPT: Implementação de `default_initializer` no Doido Framework (doido-core e doido-model)

## Contexto e Motivação

Atualmente, aplicações desenvolvidas com o framework **Doido** precisam implementar manualmente inicializadores de infraestrutura em `config/initializers/db.rs` para garantir que o pool de conexões com o banco de dados (`doido_model::pool`) esteja disponível durante o ciclo de boot dos initializers da aplicação (ex.: autenticação, agendamento de jobs, backfills).

Isso introduz complexidade desnecessária e código duplicado em cada projeto:
1. **Inspeção manual de CLI args:** A aplicação precisa inspecionar `std::env::args()` (ex.: `is_server_or_worker_cli()`) para evitar inicializar o pool durante comandos como `doido db migrate`, `doido generate` ou testes.
2. **Diferenciação Server vs. Worker:** A aplicação precisa carregar configurações distintas manualmente (`doido_model::pool::pool_options` para worker vs `doido_model::pool::init()` para server).
3. **Gambiarra de runtime assíncrono:** Como o registro de initializers roda em contexto síncrono no boot, a aplicação precisa implementar helpers como `block_on` com `tokio::task::block_in_place` ou criar runtimes temporários do Tokio.

## Objetivo

Implementar a **Opção B** no framework Doido: o próprio framework deve fornecer e cadastrar um `default_initializer` de forma nativa e modular, dividindo as responsabilidades entre **`doido-core`** e **`doido-model`**.

---

## Requisitos Arquiteturais

### 1. `doido-core`

No crate **`doido-core`** (onde residem as abstrações base de boot, runtime e lifecycle):

- **Função `default_initializer`:**
  - Exportar uma função pública `pub fn default_initializer() -> doido_core::Result<()>` (ou integrada ao registro de initializers).
  - Responsabilidade:
    - Inicializar observabilidade/logs básicos (`tracing`/`logger`), caso ainda não configurados.
    - Carregar/validar variáveis de ambiente essenciais (`.env` em modo desenvolvimento).
    - Garantir idempotência através de controle atômico (`AtomicBool`).
- **Registro Central de Initializers Padrão:**
  - No módulo de initializers (`doido_controller::initializers` ou movido/re-exportado via `doido_core`), fornecer um construtor que já traga os initializers essenciais pré-registrados:
    ```rust
    impl Initializers {
        /// Cria o registro já pré-populado com os initializers padrão do framework.
        pub fn with_defaults() -> Self;
    }
    ```

### 2. `doido-model`

No crate **`doido-model`** (onde reside a camada de banco de dados, SeaORM e pool):

- **Função `default_initializer`:**
  - Exportar uma função pública `pub fn default_initializer() -> doido_core::Result<()>` (compatível com a assinatura `FnOnce() -> Result<()>` esperada por `Initializers::register`).
  - Responsabilidade:
    1. **Identificação inteligente do contexto de execução:**
       - Detectar automaticamente se o binário está executando em modo `server`, `worker`, ou em comandos que não necessitam do pool de aplicação (como `db migrate`, `db seed`, `generate`, ou suites de testes unitários).
       - Abortar a inicialização antecipada de forma no-op (`Ok(())`) quando executado sob comandos de migração (`doido db migrate`), permitindo que o migrador gerencie sua própria conexão isolada.
    2. **Resolução de runtime assíncrono:**
       - Executar a conexão assíncrona com segurança dentro do contexto de boot (utilizando o runtime do Tokio ativo via `Handle::try_current` ou criando um bloco assíncrono seguro), sem exigir que a aplicação crie funções auxiliares de runtime.
    3. **Diferenciação transparente Server vs. Worker:**
       - Se for `worker`: carregar `doido_model::config`, extrair `db.pool`, timeout e instanciar `doido_model::pool::set_pool(conn)`.
       - Se for `server`: invocar `doido_model::pool::init().await`.
    4. **Idempotência:**
       - Garantir que múltiplas chamadas sejam seguras e ignoradas caso o pool já esteja inicializado (`doido_model::pool::try_pool().is_some()`).

### 3. Integração no `doido` (Crate Guarda-chuva)

No crate principal **`doido`**:
- Garantir que ao construir o builder:
  ```rust
  doido::Doido::new()
      .initializers(initializers::registry())
  ```
  Ou ao invocar os initializers da aplicação, o framework permita compor facilmente os defaults:
  ```rust
  // Na aplicação:
  pub fn registry() -> Initializers {
      let mut inits = Initializers::with_defaults(); // Já inclui doido-core e doido-model
      // ou:
      inits.register("core", doido_core::default_initializer);
      inits.register("db", doido_model::default_initializer);
      
      // Initializers específicos da regra de negócio da aplicação:
      inits.register("storage", storage::init);
      inits.register("auth", auth::init);
      inits.register("jobs", jobs::init);
      inits
  }
  ```

---

## Comparativo: Antes vs. Depois na Aplicação

### Antes (38 linhas de boilerplate na aplicação):
```rust
// backend/config/initializers/db.rs
pub fn init() -> doido::Result<()> {
    if !is_server_or_worker_cli() {
        return Ok(());
    }
    block_on(async {
        if is_worker_cli() {
            let config = doido_model::config::load();
            let opts = doido_model::pool::pool_options(...);
            let conn = Database::connect(opts).await?;
            doido_model::pool::set_pool(conn);
        } else {
            doido_model::pool::init().await;
        }
    });
    Ok(())
}
```

### Depois (apenas registrar o `default_initializer` do framework):
```rust
// backend/config/initializers/mod.rs
fn register_initializers(inits: &mut Initializers) {
    inits.register("core", doido_core::default_initializer);
    inits.register("settings", settings::init);
    inits.register("db", doido_model::default_initializer);
    // ... demais initializers de negócio da aplicação
}
```

---

## Critérios de Aceite

1. **Compilação e Modularidade:**
   - `doido-core` compila independentemente sem depender de `doido-model`.
   - `doido-model` exporta `pub fn default_initializer() -> doido_core::Result<()>`.
2. **Segurança de CLI e Migrações:**
   - Executar `doido db migrate` **não** inicializa o pool de aplicação via `default_initializer` e não gera pânico de conexão ou bloqueios.
3. **Compatibilidade com Worker e Server:**
   - Executar `doido server` inicializa o pool com as configurações padrão de web server.
   - Executar `doido worker` inicializa o pool com as configurações dimensionadas de worker.
4. **Testes Automatizados:**
   - Teste unitário no `doido-model` garantindo que `default_initializer()` é idempotente.
   - Teste de integração garantindo que `Initializers::with_defaults()` roda com sucesso.
