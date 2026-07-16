# curl examples

From a repository checkout with the [minimal config](/guide/getting-started) running, every endpoint has the following illustrative exchange. Timestamps are generated at runtime.

```sh
curl http://127.0.0.1:3000/v1/health
curl -X POST http://127.0.0.1:3000/v1/items -H 'content-type: application/json' -d '{"name":"tea"}'
curl http://127.0.0.1:3000/v1/items
curl http://127.0.0.1:3000/v1/items/1
curl -X PUT http://127.0.0.1:3000/v1/items/1 -H 'content-type: application/json' -d '{"name":"green tea"}'
curl -X DELETE http://127.0.0.1:3000/v1/items/1
```

```json
// GET /health → 200
{"status":"ok"}
// POST /items → 201
{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}
// GET /items → 200
[{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}]
// GET /items/1 → 200
{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}
// PUT /items/1 → 200
{"id":1,"name":"green tea","created_at":1760000000,"updated_at":1760000010}
// DELETE /items/1 → 200
{"id":1}
```

`GET` or `PUT` for a nonexistent item returns `404 {"error":"resource not found"}`. Deleting one returns JSON `null` with 200 because the action is `optional`. Missing `name` or a non-object JSON body returns 400.

The full demo additionally needs a solved ALTCHA proof for registration and Basic authentication to mint a Bearer token. Do not use test seed phrases as production credentials.
