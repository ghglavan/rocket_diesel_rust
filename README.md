# rocket_diesel_rust

to get postgres-db address
```
docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' postgres
```

Create database env:
```
echo DATABASE_URL=postgres://username:password@localhost/rdr > .env
```

```
diesel setup
```
