// Util functions
function $(html){
	return document.getElementById(html);
}

function set_cookie(name, value, days) {
  let expires = "";
  if (days) {
    const date = new Date();
    date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
    expires = "; expires=" + date.toUTCString();
  }
  document.cookie = name + "=" + encodeURIComponent(value) + expires + "; path=/";
}

function get_cookie(name) {
  const cookies = document.cookie.split('; ');
  for (let cookie of cookies) {
    const [key, value] = cookie.split('=');
    if (key === name) {
      return decodeURIComponent(value);
    }
  }
  return null;
}

function delete_cookie(name) {
  set_cookie(name, "", -1);
}

// consts
const ROOT = $("ROOT");
const NAVM = $("NAVM");

function main(){
	refresh(window.location.hash);
	for (let i=0; i<NAVM.childElementCount; i++){
		let link = NAVM.children[i];
		link.addEventListener('click', () => {
			refresh(link.getAttribute('href'));
		});
	}
}

function refresh(hash){
	let response = check_password();
	if (response == true){
		switch (hash){
			case "#dashboard":
				dashboard();
				break;
			case "#admin":
				admin_panel();
				break;
			default:
				dashboard();
				break;
		}
	}
}

function dashboard(){
	ROOT.innerHTML = "<h1>Loading...</h1>";
	fetch(`/?method=PER+0&password=${get_cookie("password")}&arg1=${get_cookie("teacher_id")}`)
	.then(response => response.text())
	.then(result => {
		ROOT.innerHTML = result
	})
	.catch(() => {
		ROOT.innerHTML = "<h1>500 - Internal Server Error</h1>"
	});
	check_password();
}

function post_login(){
	fetch(`/?method=PER+2&password=${get_cookie("password")}`)
	.then(response => response.text())
	.then(result => {
		ROOT.innerHTML = result
	})
	.catch(() => {
		ROOT.innerHTML = "<h1>500 - Internal Server Error</h1>"
	})
}

function admin_panel(){
	fetch (`/?method=PER+3&password=${get_cookie("password")}`)
	.then(result => result.text())
	.then(response => {
		ROOT.innerHTML = response;
		$("submit").addEventListener('click', () => {
			let value = $("select").value;
			fetch (`/?method=${value}&password=test`)
			.then(result => result.text())
			.then(response => {
				$("DATABASE-CONTENT").innerHTML = response;
			})
		})
	})
}

function check_password(){
	fetch (`/?method=PER+1&password=${get_cookie("password")}`)
	.then(result => result.text())
	.then(response => {
		if (response !== "true"){
			ROOT.innerHTML = response;
			$("submit_but").addEventListener('click', () => {
				set_cookie("password", $("password").value, 1);
				set_cookie("teacher_id", $("id").value, 1);
				post_login();
			})
		}
		else{
			return true;
		}
	})
	.catch(() => {
	});
	return true;
}

main();
