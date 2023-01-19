#!/usr/bin/env bash

VERSION=$(sentry-cli releases propose-version || exit)

kubectl apply -f pvc.yaml || exit
kubectl apply -f config.yaml || exit
kubectl apply -f secrets.yaml || exit
kubectl apply -f deploy.yaml || exit
