; Ruby edge captures consumed by RubyPlugin::extract_edge:
; - import.path/import.method/import.call -> imports
; - call.name/call.receiver/call.node -> calls and instantiates
; - instantiate.class -> instantiates
; - extends.parent -> extends
; - implements.module/implements.method/implements.call -> implements
; - type.name -> references_type
; - meta.method/meta.literal -> conservative literal metaprogramming

; require "json", require_relative "../lib/foo", load "file.rb"
(call
  method: (identifier) @import.method
  arguments: (argument_list
    (string) @import.path)) @import.call
  (#match? @import.method "^(require|require_relative|load)$")

; autoload :PaymentService, "payment_service"
(call
  method: (identifier) @import.method
  arguments: (argument_list
    (simple_symbol) @import.name
    (string) @import.path)) @import.call
  (#eq? @import.method "autoload")

; Direct calls, command calls, DSL calls, and calls without parentheses.
; Receiver calls also match this structural shape, so RubyPlugin validates the
; captured call text and ignores receiver/import/mixin/metaprogramming cases.
(call
  method: (_) @call.name) @call.node

; Calls with an explicit receiver: logger.info, self.foo, PaymentService.build,
; Payments::Processor.call, and safe navigation obj&.foo.
(call
  receiver: (_) @call.receiver
  method: (_) @call.name) @call.node

; ClassName.new and Payments::PaymentResult.new.
(call
  receiver: (_) @instantiate.class
  method: (identifier) @instantiate.method) @instantiate.node
  (#eq? @instantiate.method "new")

; class Child < Parent
(class
  superclass: (superclass
    [(constant) (scope_resolution)] @extends.parent)) @extends.node

; include Auditable, prepend Instrumented, extend ClassMethods
(call
  method: (identifier) @implements.method
  arguments: (argument_list
    [(constant) (scope_resolution)] @implements.module)) @implements.call
  (#match? @implements.method "^(include|prepend|extend)$")

; Constants used as receiver/argument/right-hand value references.
(call
  receiver: [(constant) (scope_resolution)] @type.name) @type.node

(argument_list
  [(constant) (scope_resolution)] @type.name) @type.node

(assignment
  right: [(constant) (scope_resolution)] @type.name) @type.node

; Constant subscript: `Klass[key]`.
(element_reference
  object: [(constant) (scope_resolution)] @type.name) @type.node

; case/when: `case x when Klass`.
(when
  pattern: (pattern
    [(constant) (scope_resolution)] @type.name)) @type.node

; Conservative literal metaprogramming only. Dynamic arguments intentionally
; do not match these patterns.
(call
  method: (identifier) @meta.method
  arguments: (argument_list
    [(simple_symbol) (string)] @meta.literal)) @meta.node
  (#match? @meta.method "^(send|public_send|__send__|define_method|const_get)$")

(super) @ruby.super

(yield) @ruby.yield
