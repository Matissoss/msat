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
	$("ADMIN").innerHTML = en_or_pl("Admin Panel", "Panel Administratora");
	$("SEARCH").innerHTML = en_or_pl("Search Data", "Wyszukaj Dane");
	$("DELETE").innerHTML = en_or_pl("Delete Data", "Usuń Dane");
	$("DASHBOARD").innerHTML = en_or_pl("Dashboard", "Ekran Wejściowy");
	$("INFO").innerHTML = en_or_pl("Information", "Informacje");
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
						break;
					case "#admin":
						admin_dashboard_comp();
						break;
					case "#search":
						search_component();
						break;
					case "#delete":
						delete_component();
						break;
					default:
						dashboard_comp();
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

function delete_component(){
	ROOT.innerHTML = `
	<div class='login'> 
		<h2 style='color:var(--accent1)'>${en_or_pl("Delete Data", "Usuń Dane")}</h2>
		<p>${en_or_pl("(Only works on unused data)", "(Działa tylko na nieużywanych danych)")}</p>
		<select id='selection'>
			<option value='0'>-</option>
			<option value='1'>${en_or_pl("Teacher", "Nauczyciela")}</option>
			<option value='2'>${en_or_pl("Subject", "Przedmiot")}</option>
			<option value='3'>${en_or_pl("Class", "Klasę(8)")}</option>
			<option value='4'>${en_or_pl("Classroom", "Klasę")}</option>
			<option value='5'>${en_or_pl("Lesson Hour", "Godzinę Lekcyjną")}</option>
			<option value='6'>${en_or_pl("Break Hour", "Przerwę")}</option>
			<option value='7'>${en_or_pl("Duty Place", "Miejsce Przerwy")}</option>
			<option value='8'>${en_or_pl("Semester", "Semestr")}</option>
			<option value='9'>${en_or_pl("Academic Year", "Rok szkolny")}</option>
			<option value='10'>${en_or_pl("Lesson", "Lekcję")}</option>
			<option value='11'>${en_or_pl("Duty", "Dyżur")}</option>
		</select>
		<div id='selection_output'>

		</div>
		<div id='results'>

		</div>
		<button id='submit'>${en_or_pl("Search", "Wyszukaj")}</button>
	</div>
	`;
	$("selection").onchange = function () {
		switch ($("selection").value){
			case "1":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+5&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "2":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Subject ID", "Identyfikator Przedmiotu")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+4&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "3":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Class ID", "Identyfikator Klasy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+2&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "5":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Lesson Hour", "Godzina Lekcyjna")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+9&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "4":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Classroom ID", "Identyfikator Klasy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+3&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "6":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Break Num", "Numer przerwy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+10&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "7":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Place ID", "Identyfikator miejsca")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+8&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "8":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Semester ID", "Identyfikator semestru")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+7&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "9":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Year ID", "Identyfikator roku")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=DELETE+6&password=${get_cookie('password')}&id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "10":
				$("selection_output").innerHTML = `
				<input id='wd' type='number' min=1 max=7 placeholder=${en_or_pl("Weekday", "Dzień tygodnia")}>
				<input id='ci' type='number' min=1 max=65535 placeholder=${en_or_pl("Class ID", "Klasa(8)")}>
				<input id='lh' type='number' min=1 max=255 placeholder=${en_or_pl("Lesson hour", "Godzina Lekcyjna")}>
				<input id='se' type='number' min=1 max=255 placeholder=${en_or_pl("Semester", "Semestr")}>
				<input id='ay' type='number' min=1 max=255 placeholder=${en_or_pl("Academic Year", "Rok szkolny")}>
				`;
				$("submit").onclick = function() {
					let wd = $("wd").value;
					let ci = $("ci").value;
					let lh = $("lh").value;
					let se = $("se").value;
					let ay = $("ay").value;
					if (wd!=null&&ci!=null&&lh!=null&&se!=null&&ay!=null){
						fetch(
						`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=DELETE+0&weekday=${wd}&class_id=${ci}&semester=${se}&academic_year=${ay}&lesson_hour=${lh}`
						).then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
				}
				break;
			case "11":
				$("selection_output").innerHTML = `
				<input id='wd' type='number' min=1 max=7 placeholder="${en_or_pl("Weekday", "Dzień Tygodnia")}">
				<input id='bn' type='number' min=1 max=255 placeholder="${en_or_pl("Break num","Przerwa")}">
				<input id='ti' type='number' min=1 max=65535 placeholder="${en_or_pl("Teacher ID", "Nauczyciel")}">
				<input id='se' type='number' min=1 max=255 placeholder="${en_or_pl("Semester", "Semestr")}">
				<input id='ay' type='number' min=1 max=255 placeholder="${en_or_pl("Academic Year", "Rok szkolny")}">
				`
				$("submit").onclick = function() {
					let wd = $("wd").value;
					let bn = $("bn").value;
					let ti = $("ti").value;
					let se = $("se").value;
					let ay = $("ay").value;
					if (wd!=null&&bn!=null&&ti!=null&&bp!=null&&se!=null&&ay!=null){
						fetch(
`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=DELETE+1&weekday=${wd}&teacher_id=${ti}&semester=${se}&academic_year=${ay}&break_num=${bn}`
						).then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"));
					}
				}
				break;
		}
	}
}
function search_component(){
	ROOT.innerHTML = `
	<div class='login'> 
		<h2 style='color:var(--accent1)'>${en_or_pl("Search Data", "Wyszukaj Dane")}</h2>
		<select id='selection'>
			<option value='0'>-</option>
			<option value='1'>${en_or_pl("Teacher", "Nauczyciel")}</option>
			<option value='2'>${en_or_pl("Subject", "Przedmiot")}</option>
			<option value='3'>${en_or_pl("Class", "Klasa(8)")}</option>
			<option value='4'>${en_or_pl("Classroom", "Klasa")}</option>
			<option value='5'>${en_or_pl("Lesson Hour", "Godzina Lekcyjna")}</option>
			<option value='6'>${en_or_pl("Break Hour", "Przerwy")}</option>
			<option value='7'>${en_or_pl("Duty Places", "Miejsca Przerwy")}</option>
			<option value='8'>${en_or_pl("Semesters", "Semestry")}</option>
			<option value='9'>${en_or_pl("Academic Years", "Lata szkolne")}</option>
		</select>
		<div id='selection_output'>

		</div>
		<div id='results'>

		</div>
		<button id='submit'>${en_or_pl("Search", "Wyszukaj")}</button>
	</div>
	`;
	$("selection").onchange = function () {
		switch ($("selection").value){
			case "1":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+4&password=${get_cookie('password')}&teacher_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "2":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Subject ID", "Identyfikator Przedmiotu")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+5&password=${get_cookie('password')}&subject_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "3":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Class ID", "Identyfikator Klasy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+6&password=${get_cookie('password')}&class_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "5":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Lesson Hour", "Godzina Lekcyjna")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+12&password=${get_cookie('password')}&lesson_hour=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "4":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Classroom ID", "Identyfikator Klasy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+7&password=${get_cookie('password')}&classroom_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "6":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Break Num", "Numer przerwy")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+11&password=${get_cookie('password')}&break_num=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "7":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Place ID", "Identyfikator miejsca")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+8&password=${get_cookie('password')}&place_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "8":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Semester ID", "Identyfikator semestru")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+10&password=${get_cookie('password')}&sem_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
			case "9":
				$('selection_output').innerHTML = `
				<input id='x' type='number' min='1' max='65535' placeholder='${en_or_pl("Year ID", "Identyfikator roku")}'>`;
				$('submit').onclick = function (){
					const x = $('x').value;
					if (x!=null){
						fetch(`/?msat/${MSAT_VERSION}&method=GET+9&password=${get_cookie('password')}&year_id=${x}`)
						.then(response => response.text())
						.then(data => {
							$('results').innerHTML = data;
						})
					}
				}
				break;
		}
	}
}

function login_component(){
	return `
	<div style='height: 80vh; display:flex; align-items:center; color:var(--accent1)'>
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
			<div style='height: 40vh; display: flex; align-items: center;'>
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
	ROOT.innerHTML = `<h1 style='text-align:center;'>${en_or_pl("Dashboard", "Ekran Wejściowy")}</h1>`;
	fetch(
	`/?msat/${MSAT_VERSION}&method=GET+2&password=${get_cookie("password")}&teacher_id=${get_cookie("teacher_id")}`)
	.then(response => response.text())
	.then(data1 => {
		ROOT.innerHTML += `<h3 style='text-align:center;'>${en_or_pl("Duties", "Dyżury")}</h3>${data1}`;
	})
}
function en_or_pl(en, pl){
	if (navigator.language == "pl-PL"){
		return pl;
	}
	else{
		return en;
	}
}

function admin_dashboard_comp(){
	ROOT.innerHTML = `
		<div class='login'>
			<h1>${en_or_pl("Manipulate Database", "Manipuluj Bazą danych")}</h1>
			<select id = 'select'>
				<option>-</option>
				<option value='get1'>${en_or_pl("Lesson Table for Class", "Plan lekcji dla klasy")}</option>
				<option value='get3'>${en_or_pl("Lesson Table for Teacher", "Plan lekcji dla nauczyciela")}</option>
				<option value='get2'>${en_or_pl("Duty List for Teacher", "Lista Dyżuru dla Nauczyciela")}</option>
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
			<div id='msg'>

			</div>
			<button id='submit'>${en_or_pl("Do operation", "Przeprowadź operację")}</button>
		</div>
	`;
	$("select").addEventListener('change', () => {
		$('msg').innerHTML = '';
		switch ($("select").value){
			case "get1":
				$("form").innerHTML = `
				<input id='ci' type='number' min=1 max=65535 placeholder='${en_or_pl("Class Id", "Identyfikator klasy")}'>
				`;
				$("submit").onclick = function () {
					const ci = $('ci').value;
					if (ci!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=GET+1&class_id=${ci}`)
						.then(response => response.text())
						.then(data => {
							$("msg").innerHTML = data;
						})
					}
				}
				break;
			case "get2":
				$("form").innerHTML = `
				<input id='ti' type='number' min=1 max=65535 placeholder=${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}>
				`
				$("submit").onclick = function(){
					let ti = $('ti').value;
					if (ti!=null){
						fetch(`/?msat/${MSAT_VERSION}&password=${get_cookie('password')}&method=GET+2&teacher_id=${ti}`)
						.then(response => response.text())
						.then(data => {
							$('msg').innerHTML = data;
						})
					}
				}
				break;
			case "get3":
				$("form").innerHTML = `
				<input id='ti' type='number' min=1 max=65535 placeholder=${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}>
				`
				$("submit").onclick = function(){
					let ti = $('ti').value;
					if (ti!=null){
						fetch(`/?msat/${MSAT_VERSION}&password=${get_cookie('password')}&method=GET+3&teacher_id=${ti}`)
						.then(response => response.text())
						.then(data => {
							$('msg').innerHTML = data;
						})
					}
				}
				break;
			case "l1":
				$("form").innerHTML = `
				<input id='wd' type='number' min=1 max=7 placeholder=${en_or_pl("Weekday", "Dzień tygodnia")}>
				<input id='ci' type='number' min=1 max=65535 placeholder=${en_or_pl("Class ID", "Klasa(8)")}>
				<input id='cl' type='number' min=1 max=65535 placeholder=${en_or_pl("Classroom ID", "Klasa")}>
				<input id='ti' type='number' min=1 max=65535 placeholder=${en_or_pl("Teacher ID", "Nauczyciel")}>
				<input id='si' type='number' min=1 max=65535 placeholder=${en_or_pl("Subject ID", "Przedmiot")}>
				<input id='lh' type='number' min=1 max=255 placeholder=${en_or_pl("Lesson hour", "Godzina Lekcyjna")}>
				<input id='se' type='number' min=1 max=255 placeholder=${en_or_pl("Semester", "Semestr")}>
				<input id='ay' type='number' min=1 max=255 placeholder=${en_or_pl("Academic Year", "Rok szkolny")}>
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
							alert(data);
						})
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"));
					}
				}
				break;
			case "y2":
				$("form").innerHTML = `
				<input id='yn' type='number' min=1 max=7 placeholder="${en_or_pl("Year Number", "Numer Roku")}">
				<input id='yn1' type='text'  placeholder="${en_or_pl("Year Name","Nazwa roku")}">
				<input id='st' type='date'   placeholder="${en_or_pl("Start", "Rozpoczęcie")}">
				<input id='en' type='date'   placeholder="${en_or_pl("End", "Zakończenie")}">
				`
				$("submit").onclick = function() {
					const date1 = new Date($("st").value).toISOString();
					const date2 = new Date($("en").value).toISOString();
					const year_number = $("yn").value;
					const year_name = $("yn1").value;
					if (year_name!=null&&year_number!=null&&date1!=null&&date2!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+2&academic_year=${year_number}&year_name=${year_name}&start_date=${date1}&end_date=${date2}`)
						.then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
				}
				break;
			case "d3":
				$("form").innerHTML = `
				<input id='wd' type='number' min=1 max=7 placeholder="${en_or_pl("Weekday", "Dzień Tygodnia")}">
				<input id='bn' type='number' min=1 max=255 placeholder="${en_or_pl("Break num","Przerwa")}">
				<input id='ti' type='number' min=1 max=65535 placeholder="${en_or_pl("Teacher ID", "Nauczyciel")}">
				<input id='se' type='number' min=1 max=255 placeholder="${en_or_pl("Semester", "Semestr")}">
				<input id='ay' type='number' min=1 max=255 placeholder="${en_or_pl("Academic Year", "Rok szkolny")}">
				<input id='bp' type='number' min=1 max=255 placeholder="${en_or_pl("Break_Place", "MiejscePrzerwy")}">
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
`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+3&weekday=${wd}&teacher_id=${ti}&semester=${se}&academic_year=${ay}&break_num=${bn}&place_id=${bp}`
						).then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
					else{
						alert(en_or_pl("Enter data", "Wstaw dane"));
					}
				}
				break;
			case "b4":
				$("form").innerHTML = `
				<input id='bn' type='number' min=1 max=255 placeholder="${en_or_pl("Break Num", "Numer Przerwy")}">
				<input id='sh' type='number' min=0 max=24 placeholder="${en_or_pl("Start Hour", "Godzina Rozpoczęcia")}">
				<input id='sm' type='number' min=0 max=60 placeholder="${en_or_pl("Start Minute", "Minuta Rozpoczęcia")}">
				<input id='eh' type='number' min=0 max=24 placeholder="${en_or_pl("End Hour", "Godzina Zakończenia")}">
				<input id='em' type='number' min=0 max=60 placeholder="${en_or_pl("End Minute", "Minuta Zakończenia")}">
				`
				$("submit").onclick = function(){
					const bn = $("bn").value;
					const sh = $("sh").value;
					const sm = $("sm").value;
					const eh = $("eh").value;
					const em = $("em").value;

					if (bn!=null&&sh!=null&&sm!=null&&eh!=null&&em!=null){
						fetch(`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+4&break_num=${bn}&start_hour=${sh}&start_minute=${sm}&end_hour=${eh}&end_minute=${em}`)
						.then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
				}
				break;
			case "s5":
				$("form").innerHTML = `
				<input id='yn' type='number' min=1 max=7 placeholder="${en_or_pl("Semester Number", "Numer Semestru")}">
				<input id='yn1' type='text'   placeholder="${en_or_pl("Semester Name","Nazwa semestru")}">
				<input id='st' type='date' placeholder="${en_or_pl("Start", "Rozpoczęcie")}">
				<input id='en' type='date' placeholder="${en_or_pl("End", "Zakończenie")}">
				`;
				$("submit").onclick = function() {
					const date1 = new Date($("st").value).toISOString();
					const date2 = new Date($("en").value).toISOString();
					const sem_number = $("yn").value;
					const sem_name = $("yn1").value;
					if (sem_name!=null&&sem_number!=null&&date1!=null&&date2!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+5&semester=${sem_number}&semester_name=${sem_name}&start_date=${date1}&end_date=${date2}`)
						.then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
				}
				break;
			case "l6":
				$("form").innerHTML = `
				<input id=bn type=number min=1 max=255 placeholder="${en_or_pl("Lesson Hour", "Numer Lekcji")}">
				<input id=sh type=number min=0 max=24 placeholder="${en_or_pl("Start Hour", "Godzina Rozpoczęcia")}">
				<input id=sm type=number min=0 max=60 placeholder="${en_or_pl("Start Minute", "Minuta Rozpoczęcia")}">
				<input id=eh type=number min=0 max=24 placeholder="${en_or_pl("End Hour", "Godzina Zakończenia")}">
				<input id=em type=number min=0 max=60 placeholder="${en_or_pl("End Minute", "Minuta Zakończenia")}">
				`
				$("submit").onclick = function(){
					const bn = $("bn").value;
					const sh = $("sh").value;
					const sm = $("sm").value;
					const eh = $("eh").value;
					const em = $("em").value;

					if (bn!=null&&sh!=null&&sm!=null&&eh!=null&&em!=null){
						fetch(`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+6&lesson_num=${bn}&start_hour=${sh}&start_minute=${sm}&end_hour=${eh}&end_minute=${em}`)
						.then(response => response.text())
						.then(data => {
							alert(data);
						})
					}
				}
				break;
			case "t7":
				$("form").innerHTML = `
				<input id=iid type=number min=1 max=65535 placeholder="${en_or_pl("Teacher ID", "Identyfikator Nauczyciela")}">
				<input id=iname type=text placeholder="${en_or_pl("Teacher Name", "Imię Nauczyciela")}">
				`
				$("submit").onclick = function(){
					const id = $("iid").value;
					const name = $("iname").value;

					if (id!=null&&name!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+7&teacher_id=${id}&teacher_name=${name}`)
						.then(response => response.text())
						.then(data => {
							alert (data);
						})
					}
				}
				break;
			case "c8":
				$("form").innerHTML = `
				<input id=iid type=number min=1 max=65535 placeholder="${en_or_pl("Class ID", "Identyfikator Klasy")}">
				<input id=iname type=text placeholder="${en_or_pl("Class Name", "Nazwa Klasy")}">
				`
				$("submit").onclick = function(){
					const id = $("iid").value;
					const name = $("iname").value;

					if (id!=null&&name!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+8&class_id=${id}&class_name=${name}`)
						.then(response => response.text())
						.then(data => {
							alert (data);
						})
					}
				}
				break;
			case "c9":
				$("form").innerHTML = `
				<input id=iid type=number min=1 max=65535 placeholder="${en_or_pl("Classroom ID", "Identyfikator Klasy")}">
				<input id=iname type=text placeholder="${en_or_pl("Classroom Name", "Nazwa Klasy")}">
				`
				$("submit").onclick = function(){
					const id = $("iid").value;
					const name = $("iname").value;

					if (id!=null&&name!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+9&classroom_id=${id}&classroom_name=${name}`)
						.then(response => response.text())
						.then(data => {
							alert (data);
						})
					}
				}
				break;
			case "s10":
				$("form").innerHTML = `
				<input id=iid type=number min=1 max=65535 placeholder="${en_or_pl("Subject ID", "Identyfikator Przedmiotu")}">
				<input id=iname type=text placeholder="${en_or_pl("Subject Name", "Nazwa Przedmiotu")}">
				`
				$("submit").onclick = function(){
					const id = $("iid").value;
					const name = $("iname").value;

					if (id!=null&&name!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+10&subject_id=${id}&subject_name=${name}`)
						.then(response => response.text())
						.then(data => {
							alert (data);
						})
					}
				}
				break;
			case "c11":
				$("form").innerHTML = `
				<input id=iid type=number min=1 max=65535 placeholder="${en_or_pl("Place ID", "Identyfikator Miejsca")}">
				<input id=iname type=text placeholder="${en_or_pl("Place Name", "Nazwa Miejsca")}">
				`
				$("submit").onclick = function(){
					const id = $("iid").value;
					const name = $("iname").value;
					if (id!=null&&name!=null){
						fetch (`/?msat/${MSAT_VERSION}&password=${get_cookie("password")}&method=POST+11&place_id=${id}&place_name=${name}`)
						.then(response => response.text())
						.then(data => {
							alert (data);
						})
					}
				}
				break;
		}
	})
}

main();
