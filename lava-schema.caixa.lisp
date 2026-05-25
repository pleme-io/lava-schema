(defcaixa
  :name
  "lava-schema"
  :kind
  :Biblioteca
  :ecosystem
  :rust-single-crate
  :package
  {:name "lava-schema"
   :version "0.1.0"
   :description "Typed schema protocol for lava architectures. GraphQL-equivalent for infrastructure: every architecture declares a strict interface (typed inputs + typed outputs) other architectures consume by typed query. Loose escape hatches where strictness doesn't apply. Authored in tatara-lisp ((deflava-interface ...)). Powers cross-architecture compile-time composition."
   :license "MIT"
   :repository "https://github.com/pleme-io/lava-schema"}
  :ci-config
  {:bump {:default-type "patch"}
   :publish {:no-verify true}}
  :workflows
  [:auto-release :pre-merge-gate :security-gate])
