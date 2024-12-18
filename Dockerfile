FROM scratch
COPY ./bin/vnix_x86_64 /vnix
COPY ./bin/vnix.store /vnix.store
ENTRYPOINT ["/vnix"]
