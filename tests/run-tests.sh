#!/bin/bash -e

EXEC_PATH=$(dirname "$(realpath "$0")")
PROJECT_PATH="$(dirname $EXEC_PATH)"
TEST_PATH="${PROJECT_PATH}/tests"
TEST_OUTPUT_DIR="/tmp/nm-test"
LOG_FILE_FMT="%(asctime)s [%(levelname)8s] \
%(message)s (%(filename)s:%(lineno)s)"
START_TIME="$(date  +%H:%M:%S)"

if [ $UID -ne 0 ];then
    SUDO="sudo"
else
    SUDO=""
fi

if [ -e $TEST_OUTPUT_DIR ];then
    rm -rf $TEST_OUTPUT_DIR
fi

mkdir $TEST_OUTPUT_DIR

cd $TEST_PATH

# Make sure the `--since` command works
sleep 1;

function build {
    cd $PROJECT_PATH
    if [ ! -e $PROJECT_PATH/target/debug/NetworkManager ];then
        cargo build
    fi
}

function before_exit {
    collect_logs
    $SUDO rm -rf /etc/NetworkManager/states 2>/dev/null || true
    $SUDO mv -f /etc/NetworkManager/states.bak \
        /etc/NetworkManager/states 2>/dev/null || true
}

function collect_logs {
    $SUDO cp -a /tmp/nipart_test_* $TEST_OUTPUT_DIR/ 2>/dev/null || true
    $SUDO journalctl --since $START_TIME > $TEST_OUTPUT_DIR/journal.log
}

function check_core_dump_and_quit {
    OUTPUT=$($SUDO coredumpctl list --since $START_TIME 2>&1 || true)

    if [ "CHK$(echo $OUTPUT|grep 'No coredumps found')" == "CHK" ];then
        echo "FAIL: Found core-dump"
        echo $OUTPUT
        $SUDO coredumpctl dump --since $START_TIME -o $TEST_OUTPUT_DIR/coredump
        exit 1
    fi
}

trap before_exit ERR EXIT

build

$SUDO mv -f /etc/NetworkManager/states/ \
    /etc/NetworkManager/states.bak 2>/dev/null || true

$SUDO ulimit -c unlimited

$SUDO pytest \
    --verbose --verbose \
    --durations=5 \
    --log-level=ERROR \
    --log-file-level=DEBUG \
    --log-file-format="$LOG_FILE_FMT" \
    --log-file-date-format="%Y-%m-%d %H:%M:%S" \
    --log-file=$TEST_OUTPUT_DIR/pytest.log \
    "$@" | tee $TEST_OUTPUT_DIR/pytest.output

check_core_dump_and_quit
