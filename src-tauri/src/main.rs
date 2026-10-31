#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{env, path::{Path, PathBuf}, sync::{Arc, Mutex}};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use crust_core::{authenticator::{Account, Authenticator, Ownership}, foundation::events::Event, launcher::{Launch, LaunchOptions, LoaderKind}};
use reqwest::{multipart::{Form, Part}, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
struct AppState { account: Mutex<Option<Account>> }
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
struct AccountView { name:String, uuid:String, can_play:bool, skins:Vec<Value>, capes:Vec<Value> }
impl From<&Account> for AccountView {
 fn from(a:&Account)->Self {
  let p=serde_json::to_value(&a.profile).unwrap_or_else(|_|json!({}));
  Self{name:a.name.clone(),uuid:a.uuid.clone(),can_play:matches!(a.meta.ownership,Ownership::Owned|Ownership::GamePass),skins:p.get("skins").and_then(Value::as_array).cloned().unwrap_or_default(),capes:p.get("capes").and_then(Value::as_array).cloned().unwrap_or_default()}
 }
}
#[derive(Serialize)] struct GameVersion{id:String,release_time:String}
#[derive(Serialize)] #[serde(rename_all="camelCase")] struct LoginCode{user_code:String,url:String,message:String}
#[derive(Debug,Deserialize)] struct ModrinthFile{filename:String,url:String,#[serde(default)]primary:bool}
#[derive(Debug,Deserialize)] struct ModrinthVersion{files:Vec<ModrinthFile>}
#[derive(Debug,Deserialize)] struct ModrinthSearchResponse{hits:Vec<ModrinthHit>}
#[derive(Debug,Deserialize)] struct ModrinthHit{project_id:String,slug:String,title:String,description:String,downloads:u64,icon_url:Option<String>,project_type:String}
#[derive(Serialize)] #[serde(rename_all="camelCase")] struct ModSearchResult{project_id:String,slug:String,title:String,description:String,downloads:u64,icon_url:Option<String>,project_type:String}
#[derive(Debug,Deserialize)] struct ModrinthInstallVersion{files:Vec<ModrinthFile>}
#[derive(Serialize)] #[serde(rename_all="camelCase")] struct ServerStatus{online:bool,host:String,version:Option<String>,players_online:Option<u64>,players_max:Option<u64>,motd:Option<String>,latency_ms:Option<u64>}

fn validate_version(v:&str)->Result<(),String>{if v.is_empty()||v.len()>32||!v.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'.'|'-'|'_')){return Err("Invalid Minecraft version identifier.".into())}Ok(())}
fn validate_memory(v:&str)->Result<&'static str,String>{match v{"1536M"=>Ok("1536M"),"2G"=>Ok("2G"),"2560M"=>Ok("2560M"),_=>Err("Unsupported memory preset.".into())}}
fn make_authenticator()->Result<Authenticator,String>{match env::var("NOVA_MICROSOFT_CLIENT_ID"){Ok(id)if !id.trim().is_empty()=>Authenticator::with_client_id(id).map_err(|e|e.to_string()),_=>Authenticator::new().map_err(|e|e.to_string())}}
fn account_snapshot(s:&State<'_,AppState>)->Result<Account,String>{s.account.lock().map_err(|_|"Account state unavailable".to_string())?.clone().ok_or_else(||"Sign in with Microsoft before using this feature.".into())}

#[tauri::command]
async fn get_versions()->Result<Vec<GameVersion>,String>{
 let r=Client::new().get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let m:Value=r.json().await.map_err(|e|e.to_string())?;let latest=m.pointer("/latest/release").and_then(Value::as_str).unwrap_or_default().to_owned();let vs=m.get("versions").and_then(Value::as_array).ok_or("Mojang version list missing")?;let mut out=Vec::new();
 if let Some(v)=vs.iter().find(|v|v.get("id").and_then(Value::as_str)==Some(latest.as_str())){out.push(GameVersion{id:latest.clone(),release_time:v.get("releaseTime").and_then(Value::as_str).unwrap_or_default().into()});}
 for v in vs.iter().filter(|v|v.get("type").and_then(Value::as_str)==Some("release")){let Some(id)=v.get("id").and_then(Value::as_str)else{continue};if id==latest{continue}out.push(GameVersion{id:id.into(),release_time:v.get("releaseTime").and_then(Value::as_str).unwrap_or_default().into()});if out.len()>=35{break}}
 if out.is_empty(){return Err("No Minecraft releases returned.".into())}Ok(out)
}
#[tauri::command]
async fn get_local_versions(root:String)->Result<Vec<GameVersion>,String>{
 let base=PathBuf::from(root.trim());if !base.is_absolute(){return Err("Minecraft folder must be an absolute path.".into())}let dir=base.join("versions");if !dir.is_dir(){return Ok(vec![])}let mut out=Vec::new();let mut entries=tokio::fs::read_dir(dir).await.map_err(|e|e.to_string())?;
 while let Some(e)=entries.next_entry().await.map_err(|e|e.to_string())?{let p=e.path();if !p.is_dir(){continue}let Some(id)=p.file_name().and_then(|v|v.to_str())else{continue};if id.len()<=80&&id.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'.'|'-'|'_'|'+'))&&tokio::fs::metadata(p.join(format!("{id}.json"))).await.is_ok(){out.push(GameVersion{id:id.into(),release_time:"Imported local profile".into()})}}
 out.sort_by(|a,b|a.id.cmp(&b.id));Ok(out)
}
#[tauri::command] async fn get_account(state:State<'_,AppState>)->Result<Option<AccountView>,String>{Ok(state.account.lock().map_err(|_|"Account state unavailable".to_string())?.as_ref().map(AccountView::from))}
#[tauri::command]
async fn sign_in(app:AppHandle,state:State<'_,AppState>)->Result<AccountView,String>{
 let flow=make_authenticator()?.device_code().await.map_err(|e|e.to_string())?;
 app.emit("login-code",LoginCode{user_code:flow.user_code().into(),url:flow.verification_uri_complete(),message:flow.message().into()}).map_err(|e|e.to_string())?;
 let account=flow.wait().await.map_err(|e|format!("Microsoft sign-in failed: {e}"))?;let view=AccountView::from(&account);*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(account);let _=app.emit("account-updated",view.clone());Ok(view)
}
#[tauri::command]
async fn search_modrinth(query:String,game_version:String,loader:String)->Result<Vec<ModSearchResult>,String>{
 validate_version(&game_version)?;if !["fabric","forge","neoforge","quilt"].contains(&loader.as_str()){return Err("Choose a supported mod loader.".into())}let q=query.trim();if q.is_empty()||q.len()>100{return Err("Enter a mod name (1–100 characters).".into())}
 let facets=json!([[format!("versions:{game_version}")],[format!("categories:{loader}")],["project_type:mod"]]).to_string();
 let r=Client::builder().user_agent("NovaClient/0.3.0").build().map_err(|e|e.to_string())?.get("https://api.modrinth.com/v2/search").query(&[("query",q),("limit","20"),("facets",facets.as_str())]).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let data:ModrinthSearchResponse=r.json().await.map_err(|e|e.to_string())?;Ok(data.hits.into_iter().map(|h|ModSearchResult{project_id:h.project_id,slug:h.slug,title:h.title,description:h.description,downloads:h.downloads,icon_url:h.icon_url,project_type:h.project_type}).collect())
}
#[tauri::command]
async fn install_modrinth_project(root:Option<String>,version:String,loader:String,project_id:String,app:AppHandle)->Result<String,String>{
 validate_version(&version)?;if !["fabric","forge","neoforge","quilt"].contains(&loader.as_str()){return Err("Unsupported mod loader.".into())}
 if project_id.len()<8||project_id.len()>64||!project_id.chars().all(|c|c.is_ascii_alphanumeric()){return Err("Invalid Modrinth project ID.".into())}
 let base=match root.filter(|s|!s.trim().is_empty()){Some(p)=>{let p=PathBuf::from(p);if !p.is_absolute(){return Err("Game folder must be absolute.".into())}p},None=>app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft").join("instances").join(format!("nova-{version}"))};
 let mods=base.join("mods");tokio::fs::create_dir_all(&mods).await.map_err(|e|e.to_string())?;let c=Client::new();let loaders=json!([loader]).to_string();let versions=json!([version]).to_string();
 let r=c.get(format!("https://api.modrinth.com/v2/project/{project_id}/version")).query(&[("loaders",loaders.as_str()),("game_versions",versions.as_str())]).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let releases:Vec<ModrinthInstallVersion>=r.json().await.map_err(|e|e.to_string())?;let f=releases.first().ok_or("No compatible release")?.files.iter().find(|f|f.primary).or_else(||releases[0].files.first()).ok_or("No downloadable file")?;
 let name=Path::new(&f.filename).file_name().and_then(|v|v.to_str()).ok_or("Invalid filename")?;if !name.to_lowercase().ends_with(".jar")||name.contains(".."){return Err("Unsafe mod filename".into())}
 let bytes=c.get(&f.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().await.map_err(|e|e.to_string())?;if bytes.len()>100*1024*1024||!bytes.starts_with(b"PK\x03\x04"){return Err("Downloaded mod is invalid or too large".into())}tokio::fs::write(mods.join(name),bytes).await.map_err(|e|e.to_string())?;Ok(name.into())
}
#[tauri::command]
async fn get_server_status(address:String)->Result<ServerStatus,String>{
 let a=address.trim();if a.is_empty()||a.len()>253||!a.chars().all(|c|c.is_ascii_alphanumeric()||matches!(c,'.'|'-'|':'|'_')){return Err("Enter a valid server hostname or IP.".into())}
 let r=Client::builder().timeout(std::time::Duration::from_secs(8)).user_agent("NovaClient/0.3.0").build().map_err(|e|e.to_string())?.get(format!("https://api.mcsrvstat.us/3/{a}")).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let d:Value=r.json().await.map_err(|e|e.to_string())?;let p=d.get("players");Ok(ServerStatus{online:d.get("online").and_then(Value::as_bool).unwrap_or(false),host:a.into(),version:d.get("version").and_then(Value::as_str).map(str::to_owned),players_online:p.and_then(|v|v.get("online")).and_then(Value::as_u64),players_max:p.and_then(|v|v.get("max")).and_then(Value::as_u64),motd:d.pointer("/motd/clean").and_then(Value::as_array).map(|a|a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")),latency_ms:None})
}
async fn install_mod(client:&Client,dir:&Path,slug:&str,version:&str)->Result<String,String>{
 let loaders=json!(["fabric"]).to_string();let versions=json!([version]).to_string();let r=client.get(format!("https://api.modrinth.com/v2/project/{slug}/version")).query(&[("loaders",loaders.as_str()),("game_versions",versions.as_str())]).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
 let vs:Vec<ModrinthVersion>=r.json().await.map_err(|e|e.to_string())?;let f=vs.first().ok_or("No compatible build")?.files.iter().find(|f|f.primary).or_else(||vs[0].files.first()).ok_or("No mod file")?;
 let n=Path::new(&f.filename).file_name().and_then(|v|v.to_str()).ok_or("Invalid filename")?;if !n.ends_with(".jar")||n.contains(".."){return Err("Unsafe filename".into())}
 let b=client.get(&f.url).send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?.bytes().await.map_err(|e|e.to_string())?;if b.len()>100*1024*1024||!b.starts_with(b"PK\x03\x04"){return Err("Invalid mod archive".into())}tokio::fs::write(dir.join(n),b).await.map_err(|e|e.to_string())?;Ok(n.into())
}
async fn performance_pack(app:&AppHandle,root:&Path,version:&str)->Result<(),String>{
 let dir=root.join("mods");tokio::fs::create_dir_all(&dir).await.map_err(|e|e.to_string())?;let c=Client::new();
 for slug in ["fabric-api","sodium","lithium","ferrite-core","immediatelyfast","entityculling","dynamic-fps"]{match install_mod(&c,&dir,slug,version).await{Ok(n)=>{let _=app.emit("launcher-progress",format!("Installed {n}"));},Err(e)=>{let _=app.emit("launcher-progress",format!("Skipped optional mod {slug}: {e}"));}}}Ok(())
}
#[tauri::command]
async fn launch_game(app:AppHandle,state:State<'_,AppState>,version:String,profile:String,memory:String,loader:String,game_root:Option<String>,server_address:Option<String>)->Result<(),String>{
 validate_version(&version)?;let memory=validate_memory(&memory)?;let kind=match loader.as_str(){"vanilla"=>None,"fabric"=>Some(LoaderKind::Fabric),"forge"=>Some(LoaderKind::Forge),"neoforge"=>Some(LoaderKind::NeoForge),"quilt"=>Some(LoaderKind::Quilt),"legacyfabric"=>Some(LoaderKind::LegacyFabric),_=>return Err("Unsupported loader".into())};
 if !["performance","custom","vanilla"].contains(&profile.as_str()){return Err("Unknown client profile".into())}
 let mut account=account_snapshot(&state)?;if !matches!(account.meta.ownership,Ownership::Owned|Ownership::GamePass){return Err("This Microsoft account does not have confirmed Java Edition access.".into())}
 if account.is_expired(){account=make_authenticator()?.refresh(&account).await.map_err(|e|e.to_string())?;*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(account.clone())}
 let root=match game_root.filter(|s|!s.trim().is_empty()){Some(p)=>{let p=PathBuf::from(p);if !p.is_absolute(){return Err("Minecraft folder must be absolute".into())}p},None=>app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft")};tokio::fs::create_dir_all(&root).await.map_err(|e|e.to_string())?;
 let name=format!("nova-{version}");let custom=game_root_is_custom(&root,&app);let mut o=LaunchOptions::new(root.to_string_lossy().to_string(),version.clone());o.instance=if custom{None}else{Some(name.clone())};o.memory.min="512M".into();o.memory.max=memory.into();o.download_concurrency=3;o.screen.width=1280;o.screen.height=720;
 if let Some(k)=kind{o.loader.kind=Some(k);o.loader.build="latest".into();o.loader.enable=true}
 if let Some(a)=server_address.filter(|s|!s.trim().is_empty()){let a=a.trim();let(host,port)=a.rsplit_once(':').filter(|(_,p)|p.parse::<u16>().is_ok()).map(|(h,p)|(h,Some(p))).unwrap_or((a,None));o.game_args.push("--server".into());o.game_args.push(host.into());if let Some(p)=port{o.game_args.push("--port".into());o.game_args.push(p.into())}}
 if profile=="performance"&&loader=="fabric"{let p=if custom{root.clone()}else{root.join("instances").join(&name)};performance_pack(&app,&p,&version).await?}
 let event_app=app.clone();let launch=Launch::new(o,account).map_err(|e|e.to_string())?.with_events(Arc::new(move|e:Event|{let line=match e{Event::Progress{downloaded,total,element}=>format!("{element}: {downloaded}/{total}"),Event::Patch(s)=>s,Event::Error(s)=>format!("Error: {s}"),_=>"Preparing game files".into()};let _=event_app.emit("launcher-progress",line);}));
 let mut child=launch.run().await.map_err(|e|format!("Minecraft could not start: {e}"))?;let _=app.emit("game-started",());child.wait().await.map_err(|e|e.to_string())?;let _=app.emit("game-exited",());Ok(())
}
fn game_root_is_custom(root:&Path,app:&AppHandle)->bool{app.path().app_data_dir().map(|b|root!=b.join("minecraft").as_path()).unwrap_or(true)}
#[tauri::command]
async fn list_mods(root:Option<String>,version:String,app:AppHandle)->Result<Vec<String>,String>{
 validate_version(&version)?;let base=match root.filter(|s|!s.trim().is_empty()){Some(p)=>PathBuf::from(p),None=>app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft").join("instances").join(format!("nova-{version}"))};let dir=if base.file_name().and_then(|v|v.to_str())==Some("mods"){base}else if base.join("mods").is_dir(){base.join("mods")}else{base};if !dir.exists(){return Ok(vec![])}
 let mut out=Vec::new();let mut it=tokio::fs::read_dir(dir).await.map_err(|e|e.to_string())?;while let Some(e)=it.next_entry().await.map_err(|e|e.to_string())?{let p=e.path();if p.is_file()&&p.extension().and_then(|v|v.to_str())==Some("jar"){if let Some(n)=p.file_name().and_then(|v|v.to_str()){out.push(n.into())}}}out.sort();Ok(out)
}
#[tauri::command]
async fn add_mod_file(root:Option<String>,version:String,filename:String,jar_base64:String,app:AppHandle)->Result<String,String>{
 validate_version(&version)?;let name=Path::new(&filename).file_name().and_then(|v|v.to_str()).ok_or("Invalid filename")?.to_owned();if name!=filename||!name.to_lowercase().ends_with(".jar")||name.contains(".."){return Err("Choose a valid .jar file".into())}
 if jar_base64.len()>80_000_000{return Err("Mod exceeds upload limit".into())}let bytes=BASE64.decode(jar_base64.as_bytes()).map_err(|_|"Invalid mod data")?;if !bytes.starts_with(b"PK\x03\x04"){return Err("File is not a valid JAR archive".into())}
 let base=match root.filter(|s|!s.trim().is_empty()){Some(p)=>PathBuf::from(p),None=>app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft").join("instances").join(format!("nova-{version}"))};let dir=if base.file_name().and_then(|v|v.to_str())==Some("mods"){base}else{base.join("mods")};tokio::fs::create_dir_all(&dir).await.map_err(|e|e.to_string())?;tokio::fs::write(dir.join(&name),bytes).await.map_err(|e|e.to_string())?;Ok(name)
}
#[tauri::command]
async fn remove_mod(root:Option<String>,version:String,filename:String,app:AppHandle)->Result<(),String>{
 validate_version(&version)?;let name=Path::new(&filename).file_name().and_then(|v|v.to_str()).ok_or("Invalid filename")?;if name!=filename||!name.to_lowercase().ends_with(".jar"){return Err("Invalid mod filename".into())}
 let base=match root.filter(|s|!s.trim().is_empty()){Some(p)=>PathBuf::from(p),None=>app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft").join("instances").join(format!("nova-{version}"))};let dir=if base.file_name().and_then(|v|v.to_str())==Some("mods"){base}else{base.join("mods")};tokio::fs::remove_file(dir.join(name)).await.map_err(|e|e.to_string())
}
async fn refresh_profile(state:&State<'_,AppState>)->Result<AccountView,String>{let a=account_snapshot(state)?;let updated=make_authenticator()?.refresh(&a).await.map_err(|e|e.to_string())?;let view=AccountView::from(&updated);*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(updated);Ok(view)}
#[tauri::command]
async fn upload_skin(state:State<'_,AppState>,png_base64:String,variant:String)->Result<AccountView,String>{
 if variant!="classic"&&variant!="slim"{return Err("Choose Classic or Slim skin model".into())}if png_base64.len()>700_000{return Err("Skin PNG must be below 512 KB".into())}let bytes=BASE64.decode(png_base64.as_bytes()).map_err(|_|"Invalid skin data")?;if !bytes.starts_with(b"\x89PNG\r\n\x1a\n"){return Err("File is not PNG".into())}
 let a=make_authenticator()?.refresh(&account_snapshot(&state)?).await.map_err(|e|e.to_string())?;*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(a.clone());
 let part=Part::bytes(bytes).file_name("nova-skin.png").mime_str("image/png").map_err(|e|e.to_string())?;let form=Form::new().text("variant",variant).part("file",part);let r=Client::new().post("https://api.minecraftservices.com/minecraft/profile/skins").bearer_auth(&a.access_token).multipart(form).send().await.map_err(|e|e.to_string())?;if !r.status().is_success(){return Err(format!("Minecraft rejected skin ({})",r.status()))}refresh_profile(&state).await
}
#[tauri::command]
async fn set_cape(state:State<'_,AppState>,cape_id:String)->Result<AccountView,String>{
 if cape_id.len()>64||!cape_id.chars().all(|c|c.is_ascii_hexdigit()||c=='-'){return Err("Invalid cape ID".into())}let a=make_authenticator()?.refresh(&account_snapshot(&state)?).await.map_err(|e|e.to_string())?;*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(a.clone());
 let p=serde_json::to_value(&a.profile).unwrap_or_else(|_|json!({}));if !p.get("capes").and_then(Value::as_array).map(|v|v.iter().any(|c|c.get("id").and_then(Value::as_str)==Some(cape_id.as_str()))).unwrap_or(false){return Err("Cape is not owned by this account".into())}
 let r=Client::new().put("https://api.minecraftservices.com/minecraft/profile/capes/active").bearer_auth(&a.access_token).json(&json!({"capeId":cape_id})).send().await.map_err(|e|e.to_string())?;if !r.status().is_success(){return Err(format!("Could not equip cape ({})",r.status()))}refresh_profile(&state).await
}
#[tauri::command]
async fn hide_cape(state:State<'_,AppState>)->Result<AccountView,String>{
 let a=make_authenticator()?.refresh(&account_snapshot(&state)?).await.map_err(|e|e.to_string())?;*state.account.lock().map_err(|_|"Account state unavailable".to_string())?=Some(a.clone());let r=Client::new().delete("https://api.minecraftservices.com/minecraft/profile/capes/active").bearer_auth(&a.access_token).send().await.map_err(|e|e.to_string())?;if !r.status().is_success(){return Err(format!("Could not hide cape ({})",r.status()))}refresh_profile(&state).await
}
fn main(){
 tauri::Builder::default().plugin(tauri_plugin_opener::init()).manage(AppState::default()).invoke_handler(tauri::generate_handler![get_versions,get_local_versions,get_account,sign_in,launch_game,list_mods,add_mod_file,remove_mod,upload_skin,set_cape,hide_cape,search_modrinth,install_modrinth_project,get_server_status]).run(tauri::generate_context!()).expect("error while running Nova Client");
}
