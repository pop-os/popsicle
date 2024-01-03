set -e

export APPIMAGE_EXTRACT_AND_RUN=1
git config --global --add safe.directory /github/workspace
apt-get update
apt-get install -y help2man libclang-dev libgtk-3-dev patchelf
wget https://github.com/TheAssassin/appimagecraft/releases/download/continuous/appimagecraft-x86_64.AppImage
chmod +x appimagecraft-x86_64.AppImage
./appimagecraft-x86_64.AppImage
