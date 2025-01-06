set -e 

# Clear Build directory
rm -rf build
# Compile msat
cd ../msat

cargo build --release --target x86_64-unknown-linux-gnu &
cargo build --release --target x86_64-unknown-linux-musl &
cargo build --release --target x86_64-pc-windows-gnu &
wait

# Make build directories - native means that file is compiled for your CPU and OS architecture
mkdir ../ci/build
mkdir ../ci/build/winx86_64
mkdir ../ci/build/linx86_64-libc
mkdir ../ci/build/linx86_64-musl
# make data dir
mkdir ../ci/build/winx86_64/data
mkdir ../ci/build/linx86_64-libc/data
mkdir ../ci/build/linx86_64-musl/data
# Move files to build directories
mv target/x86_64-unknown-linux-gnu/release/admin_dashboard ../ci/build/linx86_64-libc/admin_dashboard
mv target/x86_64-unknown-linux-musl/release/admin_dashboard ../ci/build/linx86_64-musl/admin_dashboard 
mv target/x86_64-pc-windows-gnu/release/admin_dashboard.exe ../ci/build/winx86_64/admin_dashboard.exe
# Now Application Server
mv target/x86_64-unknown-linux-gnu/release/app_server ../ci/build/linx86_64-libc/app_server 
mv target/x86_64-unknown-linux-musl/release/app_server ../ci/build/linx86_64-musl/app_server 
mv target/x86_64-pc-windows-gnu/release/app_server.exe ../ci/build/winx86_64/app_server.exe 
# Copy admin dashboard to build dirs
cp -r admin_dashboard/web ../ci/build/linx86_64-libc/web
cp -r admin_dashboard/web ../ci/build/linx86_64-musl/web
cp -r admin_dashboard/web ../ci/build/winx86_64/web

# Now copy setup/docs files to build dirs 
cd ..
cp -r docs/bundle ci/build/winx86_64/docs
cp -r docs/bundle ci/build/linx86_64-musl/docs
cp -r docs/bundle ci/build/linx86_64-libc/docs
# Copy example config.toml
cp ci/sample_data/config.toml ci/build/linx86_64-libc/data/config.toml
cp ci/sample_data/config.toml ci/build/linx86_64-musl/data/config.toml
cp ci/sample_data/config.toml ci/build/winx86_64/data/config.toml

# Make directory for compressed binaries
mkdir ci/build/release

tar -czvf ci/build/release/winx86_64.tar.gz ci/build/winx86_64 &
tar -czvf ci/build/release/linx86_64-musl.tar.gz ci/build/linx86_64-musl &
tar -czvf ci/build/release/linx86_64-libc.tar.gz ci/build/linx86_64-libc &
wait

echo Finished building project
