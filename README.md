# The developer toolkit

[![.github/workflows/release.yaml](https://github.com/derveloper/tdtk/actions/workflows/release.yaml/badge.svg)](https://github.com/derveloper/tdtk/actions/workflows/release.yaml)
[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/derveloper/tdtk)](https://rust-reportcard.xuri.me/report/github.com/derveloper/tdtk)

This is a collection of tools I use daily, all in a handy interactive cli tool.

## How to use

![](/asciicast.gif)

### Install

To install, download the latest release from the [releases page](https://github.com/derveloper/tdtk/releases) and
extract it to a directory in your path.

### Usage

Just run `tdtk`, it asks you to select a tool.

## Tools

* Secret handling for ansible vaults
* Service repo creation
  * Creates a new repo in github from a template repo

## Configuration

tdtk looks for a configuration file in the following locations, last found wins:
* `~/.config/tdtk.toml`
* `./.tdtk.toml`

### Example configuration

```toml
# ./.tdtk.toml
template_repo = "my-org/java-service-template"
spec_questions_path = "spec-questions.yml"
```

## Service tool

You can provide a template repo to use for the service tool. This repo will be used as a template for the new repo.

### DevOps

You can provide a yaml file for asking custom questions which then will be uses to generate a `.service-specs.yaml` file in the new repo.
See [`/spec-questions.yaml.sample`](/spec-questions.yaml.sample) for an example.