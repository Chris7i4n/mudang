; Class definitions. `name` may be a constant or scope_resolution (`Foo::Bar`).
(class
  name: (_) @name) @definition

; Module definitions are stored as `interface` symbols to match Scope's
; existing consultable type surface.
(module
  name: (_) @name) @definition

; Instance methods, including Ruby suffixes (`?`, `!`, `=`) and operator names.
(method
  name: (_) @name) @definition

; Singleton methods on the current class/module (`def self.build`).
; External receivers intentionally stay unsupported in symbol v1 to avoid
; creating misleading parent relationships.
(singleton_method
  object: (self)
  name: (_) @name) @definition

; Constant assignments (`DEFAULT_CURRENCY = "USD"`).
(assignment
  left: (constant) @name) @definition

; Lambdas assigned to local names (`handler = -> { ... }`).
(assignment
  left: (identifier) @name
  right: (lambda) @definition)

; Proc/lambda block calls assigned to local names (`handler = proc { ... }`,
; `handler = lambda { ... }`).
(assignment
  left: (identifier) @name
  right: (call
    method: (identifier) @ruby.proc
    block: (_)) @definition
  (#match? @ruby.proc "^(proc|lambda)$"))
