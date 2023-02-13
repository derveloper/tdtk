#!/usr/bin/env bash

tdtk -h
sleep 1
tdtk
sleep 1
echo ansible-vault view test.yaml
ansible-vault view test.yaml
exit