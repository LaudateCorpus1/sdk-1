#!/usr/bin/env bats

load utils/_

setup() {
    # We want to work from a temporary directory, different for every test.
    cd "$(mktemp -d -t dfx-e2e-XXXXXXXX)" || exit
    dfx_new hello
}

teardown() {
    dfx_stop
}

@test "bootstrap fetches candid file" {
    dfx_start
    dfx canister create --all
    dfx build
    dfx canister install hello
    ID=$(dfx canister id hello)
    PORT=$(cat .dfx/webserver-port)
    assert_command curl http://localhost:"$PORT"/_/candid?canisterId="$ID" -o ./web.txt
    assert_command diff .dfx/local/canisters/hello/hello.did ./web.txt
    assert_command curl http://localhost:"$PORT"/_/candid?canisterId="$ID"\&format=js -o ./web.txt
    # Relax diff as it's produced by two different compilers.
    assert_command diff --ignore-all-space --ignore-blank-lines .dfx/local/canisters/hello/hello.did.js ./web.txt
}

@test "forbid starting webserver with a forwarded port" {
    [ "$USE_IC_REF" ] && skip "skipped for ic-ref"

    assert_command_fail dfx bootstrap --port 8000
    assert_match "Cannot forward API calls to the same bootstrap server"
}

@test "uses local bootstrap if installed" {
    mkdir -p node_modules/@dfinity/bootstrap/dist
    echo "Hello World" > node_modules/@dfinity/bootstrap/dist/index.html

    dfx_start
    PORT=$(cat .dfx/webserver-port)
    assert_command curl http://localhost:"$PORT"/
    assert_match "Hello World"
}
