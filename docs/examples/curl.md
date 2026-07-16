# curl examples

## Prerequisites

Start the built-in [starter](/guide/getting-started). Timestamps in these responses are generated at runtime.

```sh
crudo
```

## Commands

```sh
curl http://127.0.0.1:3000/v1/health
curl -X POST http://127.0.0.1:3000/v1/items -H 'content-type: application/json' -d '{"name":"tea"}'
curl http://127.0.0.1:3000/v1/items
curl http://127.0.0.1:3000/v1/items/1
curl -X PUT http://127.0.0.1:3000/v1/items/1 -H 'content-type: application/json' -d '{"name":"green tea"}'
curl -X DELETE http://127.0.0.1:3000/v1/items/1
```

## Expected responses

`GET /health` returns `200`:

```json
{"status":"ok"}
```

`POST /items` returns `201`:

```json
{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}
```

`GET /items` returns `200`:

```json
[{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}]
```

`GET /items/1` returns `200`:

```json
{"id":1,"name":"tea","created_at":1760000000,"updated_at":1760000000}
```

`PUT /items/1` returns `200`:

```json
{"id":1,"name":"green tea","created_at":1760000000,"updated_at":1760000010}
```

`DELETE /items/1` returns `200`:

```json
{"id":1}
```

## Common responses

| Situation | Response |
|---|---|
| `GET` or `PUT` targets a missing item | `404 {"error":"resource not found"}` |
| `DELETE` targets a missing item | `200` with `null`, because the action is `optional` |
| `name` is missing | `400` |
| Body is not a JSON object | `400` |
