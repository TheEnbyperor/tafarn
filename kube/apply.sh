#!/usr/bin/env bash

VERSION=$(sentry-cli releases propose-version || exit)

kubectl apply -f pvc.yaml || exit
kubectl apply -f config.yaml || exit
kubectl apply -f secrets.yaml || exit
kubectl apply -f net.yaml || exit
kubectl apply -f svc.yaml || exit
sed -e "s/(version)/$VERSION/g" < deploy.yaml | kubectl apply -f - || exit
kubectl apply -f nginx.yaml || exit
kubectl apply -f ingress.yaml || exit
