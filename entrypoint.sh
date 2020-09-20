#!/bin/sh
set -e

if [[ $1 == "server_default" ]]
then
	/usr/local/bin/lares server --host $LARES_HOST --port $LARES_PORT --username $LARES_USERNAME --password $LARES_PASSWORD --interval $LARES_INTERVAL
else
	/usr/local/bin/lares "$@"
fi

