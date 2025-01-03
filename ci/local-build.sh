set -e 

# Clear Build directory
rm -rf build
# Compile msat
cd ../msat

cargo build --release

# Make build directories - native means that file is compiled for your CPU and OS architecture
mkdir ../ci/build
mkdir ../ci/build/local
# Move files to build directories
mv target/release/admin_dashboard ../ci/build/local/admin_dashboard
# Now Application Server
mv target/release/app_server ../ci/build/local/app_server 
# Copy admin dashboard to build dirs
cp -r admin_dashboard/web ../ci/build/local/web

# Now copy setup/docs files to build dirs 
cd ..
cp -r docs ci/build/local/docs

# Make directory for compressed binaries
mkdir ci/build/release

tar -czvf ci/build/release/local.tar.gz ci/build/local 

echo Finished building project
