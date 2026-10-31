import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import './styles.css';

const $ = (s) => document.querySelector(s);
const state = { account: null, code: null, busy: false };
function toast(message) { const n=$('#toast'); n.textContent=message; n.classList.add('show'); clearTimeout(state.timer); state.timer=setTimeout(()=>n.classList.remove('show'),3200); }
function status(message) { $('#client-status').textContent=message; }
function page(name) { document.querySelectorAll('.page').forEach(x=>x.classList.toggle('active-page',x.id===`page-${name}`)); document.querySelectorAll('.nav-item').forEach(x=>x.classList.toggle('active',x.dataset.page===name)); $('#crumb-current').textContent=name.toUpperCase(); }
function accountView(a) {
 state.account=a||null; const yes=!!a;
 $('#account-label').textContent=yes?a.name:'Connect account'; $('#account-subline').textContent=yes?`${a.name} · Microsoft linked`:'Not connected yet';
 $('#launch-note-title').textContent=yes?`Signed in as ${a.name}`:'Microsoft account required';
 $('#launch-note-sub').textContent=yes?(a.canPlay?'Java Edition access detected.':'Java Edition access not confirmed.'):'Sign in with the account that owns Java Edition.';
 $('#launch-btn').disabled=!yes||!a.canPlay||state.busy; $('#upload-skin-btn').disabled=!yes;
 $('#hide-cape-btn').disabled=!yes||!(a.capes||[]).some(c=>c.state==='ACTIVE');
 $('#skin-name').textContent=yes?a.name:'No profile connected';
 const skin=yes?(a.skins||[]).find(s=>s.state==='ACTIVE')||(a.skins||[])[0]:null;
 if(skin?.url){$('#active-skin-image').src=skin.url;$('#active-skin-image').classList.add('visible');$('#skin-empty').classList.add('hidden');$('#skin-variant').textContent=(skin.variant||'CLASSIC')+' MODEL';$('#skin-state').textContent=skin.state||'ACTIVE';}
 else{$('#active-skin-image').removeAttribute('src');$('#active-skin-image').classList.remove('visible');$('#skin-empty').classList.remove('hidden');$('#skin-variant').textContent='Waiting for account';$('#skin-state').textContent=yes?'NO SKIN':'OFFLINE';}
 const list=$('#cape-list');list.replaceChildren();$('#cape-count').textContent=`${(a?.capes||[]).length} OWNED`;
 for(const c of a?.capes||[]){const row=document.createElement('div');row.className='cape-item';const label=document.createElement('b');label.textContent=c.alias||'Minecraft Cape';const btn=document.createElement('button');btn.textContent=c.state==='ACTIVE'?'ACTIVE':'EQUIP';btn.disabled=c.state==='ACTIVE';btn.onclick=()=>cape(c.id);row.append(label,btn);list.append(row);}
 if(!yes) list.innerHTML='<div class="empty-capes"><b>Your capes live here.</b><small>Connect your Microsoft account to see owned capes.</small></div>';
}
async function signIn(){if(state.busy)return;state.busy=true;$('#login-modal').classList.add('visible');$('#login-waiting').classList.remove('hidden');$('#login-code-area').classList.add('hidden');$('#login-error').classList.add('hidden');status('Waiting for Microsoft sign-in');try{const a=await invoke('sign_in');accountView(a);$('#login-modal').classList.remove('visible');toast(a.canPlay?'Account connected.':'Account connected, Java Edition access not confirmed.');}catch(e){$('#login-waiting').classList.add('hidden');$('#login-error').textContent=String(e);$('#login-error').classList.remove('hidden');toast('Sign-in failed');}finally{state.busy=false;}}
async function loadVersions(){try{const versions=await invoke('get_versions');const s=$('#version-select');s.replaceChildren();for(const v of versions){const o=document.createElement('option');o.value=v.id;o.textContent=v.id;s.append(o);} }catch(e){$('#version-select').innerHTML='<option value="1.21.1">1.21.1 · fallback</option>';toast('Could not load version list: '+e);}}
const root=()=>$('#game-root').value.trim()||null;
async function mods(){const version=$('#version-select').value;if(!version)return;try{const names=await invoke('list_mods',{root:root(),version});$('#mods-count').textContent=names.length+' JAR FILES';const box=$('#mods-list');box.replaceChildren();if(!names.length){box.innerHTML='<div class="empty-capes"><b>No mods found.</b><small>Add compatible .jar files for the selected loader.</small></div>';return;}for(const filename of names){const row=document.createElement('div');row.className='mod-row';const label=document.createElement('span');label.textContent=filename;const b=document.createElement('button');b.textContent='REMOVE';b.onclick=async()=>{try{await invoke('remove_mod',{root:root(),version,filename});mods();}catch(e){toast(String(e));}};row.append(label,b);box.append(row);}}catch(e){toast('Could not read mods: '+e);}}
async function launch(){if(!state.account?.canPlay||state.busy)return;state.busy=true;$('#launch-btn').disabled=true;$('#progress-wrap').classList.add('visible');$('#progress-label').textContent='PREPARING GAME FILES';try{await invoke('launch_game',{version:$('#version-select').value,profile:$('#profile-select').value,memory:$('#memory-select').value,loader:$('#loader-select').value,gameRoot:root(),serverAddress:$('#server-address').value.trim()||null});toast('Game process exited.');}catch(e){toast('Launch failed: '+e);}finally{state.busy=false;$('#launch-btn').disabled=!state.account?.canPlay;}}
async function uploadMod(file){if(!file)return;if(!file.name.endsWith('.jar')||file.size>60*1024*1024){toast('Choose a .jar file smaller than 60 MB.');return;}try{const bytes=new Uint8Array(await file.arrayBuffer());let s='';for(const b of bytes)s+=String.fromCharCode(b);await invoke('add_mod_file',{root:root(),version:$('#version-select').value,filename:file.name,jarBase64:btoa(s)});await mods();toast('Mod added. Verify version/loader compatibility.');}catch(e){toast('Could not add mod: '+e);}$('#mod-file').value='';}
async function searchMods(){const query=$('#mod-search').value.trim();if(!query)return;const box=$('#mod-search-results');box.textContent='Searching Modrinth…';try{const hits=await invoke('search_modrinth',{query,gameVersion:$('#version-select').value,loader:$('#loader-select').value});box.replaceChildren();for(const hit of hits){const row=document.createElement('article');row.className='discover-mod-card';const info=document.createElement('div');info.className='discover-mod-info';const title=document.createElement('b');title.textContent=hit.title;const desc=document.createElement('small');desc.textContent=hit.description||'';info.append(title,desc);const b=document.createElement('button');b.textContent='INSTALL';b.onclick=async()=>{b.disabled=true;try{await invoke('install_modrinth_project',{root:root(),version:$('#version-select').value,loader:$('#loader-select').value,projectId:hit.projectId});b.textContent='INSTALLED';await mods();}catch(e){b.disabled=false;b.textContent='RETRY';toast(String(e));}};row.append(info,b);box.append(row);}if(!hits.length)box.textContent='No compatible mods found.';}catch(e){box.textContent='Search failed: '+e;}}
async function uploadSkin(file){if(!file||!state.account)return;if(file.type!=='image/png'||file.size>524288){toast('Choose a PNG skin smaller than 512 KB.');return;}const bytes=new Uint8Array(await file.arrayBuffer());let s='';for(const b of bytes)s+=String.fromCharCode(b);try{accountView(await invoke('upload_skin',{pngBase64:btoa(s),variant:$('#skin-variant-select').value}));toast('Skin updated.');}catch(e){toast('Skin update failed: '+e);}}
async function cape(id){try{accountView(await invoke('set_cape',{capeId:id}));}catch(e){toast(String(e));}}
async function hideCape(){try{accountView(await invoke('hide_cape'));}catch(e){toast(String(e));}}
async function checkServer(){const address=$('#server-address').value.trim();if(!address)return;$('#server-status').textContent='Checking…';try{const s=await invoke('get_server_status',{address});$('#server-status').textContent=s.online?`ONLINE · ${s.playersOnline??'?'} / ${s.playersMax??'?'} players · ${s.version||''}`:'OFFLINE';}catch(e){$('#server-status').textContent='Status unavailable';}}
async function init(){
 document.querySelectorAll('.nav-item').forEach(b=>b.onclick=()=>page(b.dataset.page));
 $('#login-btn').onclick=signIn;$('#account-shortcut').onclick=signIn;$('#close-login').onclick=()=>$('#login-modal').classList.remove('visible');
 $('#launch-btn').onclick=launch;$('#mod-file').onchange=e=>uploadMod(e.target.files?.[0]);$('#mod-search-btn').onclick=searchMods;$('#mod-search').onkeydown=e=>{if(e.key==='Enter')searchMods();};
 $('#server-check').onclick=checkServer;$('#version-select').onchange=mods;$('#game-root').onchange=mods;
 $('#loader-select').onchange=()=>{if($('#profile-select').value==='performance'&&$('#loader-select').value!=='fabric')$('#profile-select').value='custom';};
 $('#profile-select').onchange=()=>{if($('#profile-select').value==='performance')$('#loader-select').value='fabric';if($('#profile-select').value==='vanilla')$('#loader-select').value='vanilla';};
 $('#upload-skin-btn').onclick=()=>$('#skin-file').click();$('#skin-file').onchange=e=>uploadSkin(e.target.files?.[0]);$('#hide-cape-btn').onclick=hideCape;
 $('#open-microsoft').onclick=()=>state.code?.url&&openUrl(state.code.url);
 $('#copy-code').onclick=()=>navigator.clipboard.writeText($('#device-code').textContent).then(()=>toast('Code copied.'));
 await listen('login-code',e=>{state.code=e.payload;$('#device-code').textContent=e.payload.userCode;$('#login-waiting').classList.add('hidden');$('#login-code-area').classList.remove('hidden');});
 await listen('launcher-progress',e=>{$('#progress-wrap').classList.add('visible');$('#progress-label').textContent=String(e.payload).slice(0,60).toUpperCase();});
 await listen('game-started',()=>{status('Minecraft is running');$('#progress-label').textContent='MINECRAFT IS RUNNING';});
 await loadVersions();await mods();try{const a=await invoke('get_account');if(a)accountView(a);}catch{}
 $('#platform-label').textContent=navigator.userAgent.includes('Windows')?'WINDOWS':navigator.userAgent.includes('Mac')?'MACOS':'LINUX / DESKTOP';
}
init().catch(e=>{console.error(e);toast('Startup issue: '+e);});
