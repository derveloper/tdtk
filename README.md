# The developer toolkit

This is a collection of tools I use daily, all in a handy interactive cli tool.

## How to use

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
```