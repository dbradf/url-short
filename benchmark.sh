#!/usr/bin/env bash

TARGET_URL="evergreen.mongodb.com/task/mongodb_mongo_master_enterprise_rhel_80_64_bit_dynamic_required_replica_sets_max_mirroring_3_enterprise_rhel_80_64_bit_dynamic_required_5b10b587b11dcf21a10406ad2ad6753e1e3a983e_21_05_08_11_42_12/0"

insert() {
    xh localhost:8080 url=$TARGET_URL --print=b --ignore-stdin
}

get() {
    xh localhost:8080/$1 >/dev/null
}

workload() {
    local reads=15
    local short=$(insert)

    i=0
    while [[ "$i" -lt "$reads" ]]; do
        get $short &
        ((i = i + 1))
    done
}


i=0
while [[ "$i" -lt "$1" ]]; do
    workload &
    ((i = i + 1))
done

wait
