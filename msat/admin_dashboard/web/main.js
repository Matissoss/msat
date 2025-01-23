// Util functions
const MSAT_VERSION = 50;
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
	for (let i=0; i<NAVM.childElementCount; i++){
		let link = NAVM.children[i];
		if (link.getAttribute('href') != undefined){
			link.addEventListener('click', () => {
				refresh(link.getAttribute('href'));
			});
		}
	}
}

function refresh(hash){
	ROOT.innerHTML = "";
	fetch(`/?msat/${MSAT_VERSION}&method=PAS+0&password=${get_cookie("password")}`)
	.then(response => response.text())
		.then(data => {
			if (data === "true"){
				switch (hash){
					case "#info":
						info_component();
					break;
					case "#dashboard":
						dashboard_comp();
					case "#admin":
						admin_dashboard_comp();
					break;
				}
			}
			else{
				ROOT.innerHTML = login_component();
				$("submit").addEventListener('click', () => {
					if ($("pid").value != null && $("tid").value != null){
						set_cookie("password", $("pid").value, 1);
						set_cookie("teacher_id", $("tid").value, 1);
						refresh("#info");
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"))
					}
				});
			}
		})
	.catch(error => {
		alert(error);
	});
}

function check_password(password){
	fetch(`/?msat/${MSAT_VERSION}&method=PAS+0&password=${password}`)
	.then(response => response.text())
		.then(data => {
			if (data === "true"){
				return true;
			}
			else{
				return false;
			}
		})
	.catch(error => {
		alert(error);
	});
	return false;
}

function login_component(){
	return `
	<div style='height: 80vh; display:flex; align-items:center;'>
	<div class='login'>
		<h2 style='color:var(--accent1)'>${en_or_pl("Log in", "Zaloguj się")}</h2>
		<input type='number' min=1 max=65535 placeholder='${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}' id='tid'>
		<input type='password' placeholder='${en_or_pl("Password", "Hasło")}' id='pid'>
		<button id='submit'>Submit</button>
	</div>
	</div>
	`;
}

function info_component(){
	fetch(`/?msat/${MSAT_VERSION}&method=GET+4&password=${get_cookie("password")}&teacher_id=${get_cookie("teacher_id")}`)
	.then(response => response.text())
	.then(data => {
		if (data.startsWith("<error>") === false){
			ROOT.innerHTML = `
			<div style='height: 80vh; display: flex; align-items: center;'>
				<div class='login'>
				<h1>${en_or_pl("Info", "Informacje")}</h1>
				<p>${en_or_pl("You are logged in as:", "Jesteś zalogowany/-a jako:")} ${data}</p>
				<p>${en_or_pl("msat version 0.5/50-Boron", "wersja msat 0.5/50-Bor")}</p>
				<p><a href='https://github.com/Matissoss' target='_blank'>${en_or_pl("Created by MateusDev", "Stworzona przez MateusDev")}</a></p>
				<button id='logout'>${en_or_pl("Logout", "Wyloguj się")}</button>
				</div>
			</div>`
			$("logout").addEventListener('click', () => {
				delete_cookie('password');
				delete_cookie('teacher_id');
				alert(en_or_pl("Succesfully logged out", "Wylogowano się pomyślnie"));
				refresh("#info")
			});
		}
		else{
			ROOT.innerHTML = `
				<div style='height: 80vh; display: flex; align-items: center;'>
				${data}
				</div>
			`;
		};
	})
	.catch(error => {
		ROOT.innerHTML = error;
	});
}

function dashboard_comp(){
	fetch(`/?msat/${MSAT_VERSION}&method=GET+3&password=${get_cookie("password")}&teacher_id=${get_cookie("teacher_id")}`)
	.then(response => response.text())
	.then(data => {
		ROOT.innerHTML = data;
		fetch(
		`/?msat/${MSAT_VERSION}&method=GET+2&password=${get_cookie("password")}&teacher_id=${get_cookie("teacher_id")}`)
		.then(response => response.text())
		.then(data1 => {
			ROOT.innerHTML += data1;
		})
	})
	.catch(error => {
		ROOT.innerHTML = `${en_or_pl("Error occured", "Wystąpił błąd")}: ${error}`;
	})
}

function admin_dashboard_comp(){
	admin_dashboard_add();
}

function en_or_pl(en, pl){
	if (navigator.language == "pl-PL"){
		return pl;
	}
	else{
		return en;
	}
}

function admin_dashboard_add(){
	ROOT.innerHTML = `
	<div style='height: 80vh; display:flex;align-contents:center;'>
		<div class='login'>
			<h1>${en_or_pl("Add data to database", "Dodaj dane do bazy danych")}</h1>
			<select id = 'select'>
				<option>-</option>
				<option value='l1'>${en_or_pl("Add lessons", "Dodaj Lekcję")}</option>
				<option value='d3'>${en_or_pl("Add duty", "Wstaw dyżur")}</option>
				<option value='y2'>${en_or_pl("Add academic year", "Wstaw rok szkolny")}</option>
				<option value='b4'>${en_or_pl("Add break hour", "Wstaw przerwę")}</option>
				<option value='s5'>${en_or_pl("Add semester", "Wstaw semestr")}</option>
				<option value='l6'>${en_or_pl("Add lesson hour", "Wstaw godzinę lekcyjną")}</option>
				<option value='t7'>${en_or_pl("Add teacher", "Wstaw Nauczyciela")}</option>
				<option value='c8'>${en_or_pl("Add class", "Wstaw klasę (np. 8)")}</option>
				<option value='c9'>${en_or_pl("Add classroom", "Wstaw klasę (np. informatyczną)")}</option>
				<option value='s10'>${en_or_pl("Add subject", "Wstaw przedmiot")}</option>
				<option value='c11'>${en_or_pl("Add break place", "Wstaw miejsce przerwy")}</option>
			</select>
			<div id='form' style='display:flex;flex-direction:column'>

			</div>
			<button id='submit'>${en_or_pl("Add data", "Wstaw dane")}</button>
		</div>
	</div>
	`;
	$("select").addEventListener('change', () => {
		switch ($("select").value){
			case "l1":
				$("form").innerHTML = `
				<input id=wd type=number min=1 max=7 placeholder=${en_or_pl("Weekday", "Dzień tygodnia")}>
				<input id=ci type=number min=1 max=65535 placeholder=${en_or_pl("Class ID", "Klasa(8)")}>
				<input id=cl type=number min=1 max=65535 placeholder=${en_or_pl("Classroom ID", "Klasa")}>
				<input id=ti type=number min=1 max=65535 placeholder=${en_or_pl("Teacher ID", "Nauczyciel")}>
				<input id=si type=number min=1 max=65535 placeholder=${en_or_pl("Subject ID", "Przedmiot")}>
				<input id=lh type=number min=1 max=255 placeholder=${en_or_pl("Lesson hour", "Godzina Lekcyjna")}>
				<input id=se type=number min=1 max=255 placeholder=${en_or_pl("Semester", "Semestr")}>
				<input id=ay type=number min=1 max=255 placeholder=${en_or_pl("Academic Year", "Rok szkolny")}>
				`;
				$("submit").onclick = function() {
					let wd = $("wd").value;
					let ci = $("ci").value;
					let cl = $("cl").value;
					let ti = $("ti").value;
					let si = $("si").value;
					let lh = $("lh").value;
					let se = $("se").value;
					let ay = $("ay").value;
					if (wd!=null&&ci!=null&&cl!=null&&ti!=null&&si!=null&&lh!=null&&se!=null&&ay!=null){
						fetch(
						`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+1&weekday=${wd}&class_id=${ci}&classroom_id=${cl}&teacher_id=${ti}&subject_id=${si}&semester=${se}&academic_year=${ay}&lesson_hour=${lh}`
						).then(response => response.text())
						.then(data => {
							if (data === "msat/201-Created"){
								alert(en_or_pl("Succesfully inserted data!", "Pomyślnie dodano dane!"));
							}
							else{
								alert(data);
							}
						})
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"));
					}
				}
				break;
			case "d3":
				$("form").innerHTML = `
				<input id=wd type=number min=1 max=7 placeholder="${en_or_pl("Weekday", "Dzień Tygodnia")}">
				<input id=bn type=number min=1 max=255 placeholder="${en_or_pl("Break num","Przerwa")}">
				<input id=ti type=number min=1 max=65535 placeholder="${en_or_pl("Teacher ID", "Nauczyciel")}">
				<input id=se type=number min=1 max=255 placeholder="${en_or_pl("Semester", "Semestr")}">
				<input id=ay type=number min=1 max=255 placeholder="${en_or_pl("Academic Year", "Rok szkolny")}">
				<input id=bp type=number min=1 max=255 placeholder="${en_or_pl("Break_Place", "MiejscePrzerwy")}">
				`
				$("submit").onclick = function() {
					let wd = $("wd").value;
					let bn = $("bn").value;
					let ti = $("ti").value;
					let bp = $("bp").value;
					let se = $("se").value;
					let ay = $("ay").value;
					if (wd!=null&&bn!=null&&ti!=null&&bp!=null&&se!=null&&ay!=null){
						fetch(
						`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+2&weekday=${wd}&teacher_id=${ti}&semester=${se}&academic_year=${ay}&break_num=${bn}&place_id=${bp}`
						).then(response => response.text())
						.then(data => {
							if (data === "msat/201-Created"){
								alert(en_or_pl("Succesfully inserted data!", "Pomyślnie dodano dane!"));
							}
							else{
								alert(data);
							}
						})
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"));
					}
				}
				break;
			case "y2":
				break;
			case "b4":
				break;
			case "s5":
				break;
			case "l6":
				break;
			case "t7":
				break;
			case "c8":
				break;
			case "c9":
				break;
			case "s10":
				break;
			case "c11":
				break;
		}
	})
}

main();
