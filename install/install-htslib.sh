#!/usr/bin/env bash
set -e
set -o pipefail

# Ensure the script is run with root privileges
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <PROFILE_FILE> <INSTALL_DIR>"
    exit 1
fi

PROFILE_FILE="$1"
INSTALL_DIR="$2"

SCRIPT_DIR="$( cd "$( dirname "$0" )" && pwd )"
# exports append_if_missing
source "$SCRIPT_DIR/utils.sh"


# Check if bgzip is already installed and skip installation if found
if command -v bgzip > /dev/null; then
    echo "bgzip is already installed. Skipping installation."
    exit 0
fi

# Set the local installation directory
echo "Installing HTSlib to $INSTALL_DIR"

# Create a temporary directory
TEMP_DIR=$(mktemp -d)
echo "Created temporary directory: $TEMP_DIR"

# Change to the temporary directory
cd $TEMP_DIR

# Download htslib 1.21 source code from the official release
HTSLIB_VERSION="1.21"
HTSLIB_URL="https://github.com/samtools/htslib/releases/download/${HTSLIB_VERSION}/htslib-${HTSLIB_VERSION}.tar.bz2"
echo "Downloading HTSlib version $HTSLIB_VERSION from $HTSLIB_URL..."
wget $HTSLIB_URL

# Extract the downloaded tar.bz2 file
echo "Extracting htslib-${HTSLIB_VERSION}.tar.bz2..."
tar -xvjf htslib-${HTSLIB_VERSION}.tar.bz2

# Change to the extracted htslib directory
cd htslib-${HTSLIB_VERSION}

# Run autoreconf to build the configure script (if needed)
echo "Running autoreconf to generate configure script..."
autoreconf -i

# Configure the build environment with the local installation directory
echo "Running ./configure with prefix=$INSTALL_DIR..."
./configure --prefix=$INSTALL_DIR --libdir=$INSTALL_DIR/lib

# Compile and install htslib
echo "Compiling HTSlib..."
make

echo "Installing HTSlib locally to $INSTALL_DIR..."
make install

# Add the installation directory to the PATH if it's not already in PATH
append_if_missing "export PATH=$INSTALL_DIR/bin:\$PATH" "$PROFILE_FILE"
export PATH=$INSTALL_DIR/bin:$PATH

# ensure that the shared libraries are available
append_if_missing "export LD_LIBRARY_PATH=$INSTALL_DIR/lib:\$LD_LIBRARY_PATH" "$PROFILE_FILE"
export LD_LIBRARY_PATH=$INSTALL_DIR/lib:$LD_LIBRARY_PATH

# Clean up by removing the temporary directory
echo "Cleaning up temporary files..."
rm -rf $TEMP_DIR

# Verify the installation of bgzip and tabix
echo "Verifying installation..."

# if exit code was 0 and bgzip output contains the expected version string
bash -c '
    bgzip_output=$(bgzip --version)
    exit_code=$?

    # if exit code was 0 and bgzip output contains the expected version string
    if [[ $exit_code -eq 0 ]] && [[ $bgzip_output == *"bgzip"* ]]; then
        echo "bgzip installed successfully."
    else
        echo "bgzip installation failed."
        exit 1
    fi
'

exit_code=$?

if [ $exit_code -eq 0 ]; then
    echo "HTSlib installation completed successfully."
else
    echo "HTSlib installation failed."
    exit 1
fi