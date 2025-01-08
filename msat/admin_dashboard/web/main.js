function $(html){
	return document.getElementById(html);
}

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
	switch (hash){
		case "#dashboard":
			dashboard();
			break;
		case "#login":
			login();
			break;
		case "#admin":
			admin_panel();
			break;
		default:
			dashboard();
			break;
	}
}

function dashboard(){
	ROOT.innerHTML = "This is header";
	fetch("/?method=PER+0&password=test")
	.then(response => response.text())
	.then(result => {
		ROOT.innerHTML = result
	})
	.catch(() => {
		ROOT.innerHTML = "<h1>500 - Internal Server Error</h1>"
	})
}

function login(){
	ROOT.innerHTML = "Hello 1"
}

function admin_panel(){
	ROOT.innerHTML = "Hello 2"
}

main();
