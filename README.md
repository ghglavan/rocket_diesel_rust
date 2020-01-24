# rocket_diesel_rust

Im not proud of this code. It is top 3 worst code i've ever written considering reusability and scalability, but it works and amazingly, it wasnt full of bugs (rust played a big part here) for a 4-day project.


get postgres docker
```
docker run --name some-postgres -e POSTGRES_PASSWORD=mysecretpassword -d postgres
```

you need to have postgresql-client and diesel installed

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

run migration
```
diesel migration run
```

# Secure the database:

Some triggers are already present in up.sql. You can get them by
```
psql -p 5432 -W -U docker-container -h docker-ip

\c rdr
SELECT * FROM audit.logged_actions;
```

To enable file logging:

Get config file path:
```
psql -p 5432 -W -U docker-container -h docker-ip -c 'SHOW config_file'
```

Copy the config file so we can edit it (we dont have vim in docker):
```
docker cp container-name:/var/lib/postgresql/data/postgresql.conf .
```

Change owner (if u ran docker with sudo) so we can edit the file
```
sudo chown your-user postgresql.conf
```

edit `log_destination` to `'stderr'`, `logging_collector` to `on`,  `log_directory` to `'log'`, `log_connections` to `on`, `log_disconnections` to `on`, `log_duration` to `on` and `log_line_prefix` to `'%u %m [%p] '`

Copy the config file back
```
docker cp postgresql.conf container-name:/var/lib/postgresql/data/postgresql.conf
```

Restart docker image:
```
docker restart conrainer-name
```

A new user (rdr_user) is created in `up.sql` so we need to only allow this user to access our database. To do this, we need to update `pg_hba.conf` in the same way as `postgresql.conf`

Comment everything from `pg_hba.conf` and add 
```
local   all             all                                     trust
host    rdr             rdr_user        172.17.0.0/16           password
host     all              all             0.0.0.0/0         reject
```
if you need to use diesel migrate, you neet to add the following line at the top of the rules:
```
host    all             postgres        172.17.0.0/16           password   
```

copy it back in container and we are done. Now you can only connect using the previous user (some-postgres) using unix sockets (from inside docker). To do that, run a bash in our docker:

```
sudo docker exec -it postgres bash
```

and now connect to postgres:

```
psql -U postgres
```

`pg_hba.conf` and `postgresql.conf` are here for references for the previous steps.