# Core concepts

An **endpoint** maps an HTTP method and path to a named **action**. An action executes configured SQL with its `params` bound in order and serializes the result as JSON. No implicit CRUD exists: every route and query is deliberate.

Request values merge in this order: query parameters, then path parameters (path wins on duplicate keys), then JSON-object body (body wins). Path/query values are strings. The body must be a JSON object. Authentication adds `$owner`; an action that lists `$token` in `params` receives a fresh random Argon2 salt string.

Actions may hash named string fields with Argon2 before binding. A wallet stage first executes a required `one` primary action, derives addresses using returned columns, persists each address, and commits all work together. Any failure rolls it back.

Database rows become objects: integer/float/bool types stay JSON scalars, blobs/`BYTEA` become Base64 strings, null remains null, and other types become strings.
