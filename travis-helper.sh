#!/bin/bash

action="$1"

if [ "$action" = "install_deps" ]; then
  # Install rustfmt and clippy.
  if [[ "$TRAVIS_OS_NAME" == "linux" && "$TRAVIS_RUST_VERSION" == "stable" ]]; then
    rustup component add rustfmt-preview clippy-preview
  fi

# Build the project with default features.
elif [ "$action" = "build" ]; then
  cargo build --verbose

# Package the crate for crates.io distribution.
elif [ "$action" = "package" ]; then
  cargo package --verbose

# Run unit and integration tests.
elif [ "$action" = "test" ]; then
  cargo test --verbose

# Run Clippy.
elif [ "$action" = "clippy_run" ]; then
  if [[ "$TRAVIS_RUST_VERSION" == "stable" && "$TRAVIS_OS_NAME" == "linux" ]]; then
    cargo clippy --verbose
  fi

# Check formatting.
elif [ "$action" = "fmt_run" ]; then
  if [[ "$TRAVIS_RUST_VERSION" == "stable" && "$TRAVIS_OS_NAME" == "linux" ]]; then
      cargo fmt --verbose -- --check
  fi

# Upload code coverage report for stable builds in Linux.
elif [ "$action" = "upload_code_coverage" ]; then
  if [[ "$TRAVIS_OS_NAME" == "linux" && "$TRAVIS_RUST_VERSION" == "stable" ]]; then
    wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz &&
    tar xzf master.tar.gz &&
    cd kcov-master &&
    mkdir build &&
    cd build &&
    cmake .. &&
    make &&
    sudo make install &&
    cd ../.. &&
    rm -rf kcov-master &&
    for file in target/debug/fel_cli*[^\.d]; do mkdir -p "target/cov/$(basename $file)"; kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file"; done &&
    bash <(curl -s https://codecov.io/bash) &&
    echo "Uploaded code coverage"
  fi

# Upload development documentation for the develop branch.
elif [ "$action" = "upload_documentation" ]; then
  if [[ "$TRAVIS_OS_NAME" == "linux" && "$TRAVIS_RUST_VERSION" == "stable" && "$TRAVIS_PULL_REQUEST" = "false" && "$TRAVIS_BRANCH" == "develop" ]]; then
    cargo rustdoc -- --document-private-items &&
    echo "<meta http-equiv=refresh content=0;url=fel_cli/index.html>" > target/doc/index.html &&
    git clone https://github.com/davisp/ghp-import.git &&
    ./ghp-import/ghp_import.py -n -p -f -m "Documentation upload" -r https://"$GH_TOKEN"@github.com/"$TRAVIS_REPO_SLUG.git" target/doc &&
    echo "Uploaded documentation"
  fi

fi

exit $?