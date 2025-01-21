set -e 

# VARIABLES
_build="build"
_rust_target=("x86_64-unknown-linux-gnu" "x86_64-unknown-linux-musl" "x86_64-pc-windows-gnu")
_export_target=("linx86_64-libc" "linx86_64-musl" "winx86_64")
_http_server="admin_dashboard"
_app_server="app_server"
_local="localbuild"
_web="web"

echo "COMPILATION VARIABLES"
echo "build directory: " $_build
echo "global targets : " ${_rust_target[@]}
echo "export targets : " $_export_target
echo "http_server dir: " $_http_server
echo "app_server dir : " $_app_server
echo "localbuild dir : " $_local
echo "web directory  : " $_web
echo "================"

# FUNCTIONS

localbuild_msat() {
	cd ../msat
	cargo build --release
	mkdir ../ci/$_local
	mkdir ../ci/$_local/data
	mv target/release/$_http_server ../ci/$_local/$_http_server
	mv target/release/$_app_server ../ci/$_local/$_app_server
	cp -r $_http_server/$_web ..ci/$_local/$_web
	cp -r docs/bundle ci/$_local/docs
	tar -czvf ../ci/$_local.tar.gz ../ci/$_local
}

globalbuild_msat() {
	cd ../msat 
	for target in "${_rust_target[@]}"; do 
		cargo build --release --target $target
	done
	mkdir ../ci/$_build 
	mkdir ../ci/$_build/release
	for index in "${!_export_target[@]}"; do 
		mkdir ../ci/$_build/${_export_target[$index]}
		mkdir ../ci/$_build/${_export_target[$index]}/data
		if [[ "${_rust_target[$index]}" == "x86_64-pc-windows-gnu" ]]; then
			mv target/${_rust_target[$index]}/release/$_http_server.exe     ../ci/$_build/${_export_target[$index]}/$_http_server.exe
			mv target/${_rust_target[$index]}/release/$_app_server.exe      ../ci/$_build/${_export_target[$index]}/$_app_server.exe
		else
			mv target/${_rust_target[$index]}/release/$_http_server         ../ci/$_build/${_export_target[$index]}/$_http_server
			mv target/${_rust_target[$index]}/release/$_app_server          ../ci/$_build/${_export_target[$index]}/$_app_server
		fi
		cp -r $_http_server/$_web        ../ci/$_build/${_export_target[$index]}/$_web
		cp -r ../docs/bundle             ../ci/$_build/${_export_target[$index]}/docs
		
		tar -czvf ../ci/$_build/release/${_export_target[$index]}.tar.gz ../ci/$_build/${_export_target[$index]}
	done
}

# START
rm -rf $_build

echo "Choose build option"
echo "[ 1 ] - msat global build"
echo "[ 2 ] - msat local build"
echo "[ x ] - abort"

read input
if [[ $input == "1" ]]; then
	globalbuild_msat
	echo "Release files can be found in directory:" ci/$_build/release
elif [[ $input == "2" ]]; then
	localbuild_msat
	echo "File can be found in directory:" ci/$_local.tar.gz
else
	exit 0
fi
