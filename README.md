# lava-schema

Typed schema protocol for lava architectures — the GraphQL-equivalent for
infrastructure.

Every architecture declares a strict interface (typed inputs, typed outputs)
that other architectures consume by typed query, with loose escape hatches
where strictness genuinely doesn't apply. This is what powers
**cross-architecture composition at compile time** rather than at apply time.

## Install

```toml
[dependencies]
lava-schema = "0.1"
```

## Authoring

Interfaces are authored in tatara-lisp:

```lisp
(deflava-interface ...)
```

## The suite

```
lava-types ──► lava-schema ──► lava-eval ──► lava-runtime
```

Depends on [`lava-types`](https://github.com/pleme-io/lava-types) for its
constraint validators.

## License

MIT
