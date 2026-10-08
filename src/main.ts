import '@fontsource/vt323/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-600.css';
import './styles/main.css';
import { app } from './game';
import { ui } from './ui/root';
import { hud } from './ui/hud';
import { mountTouchControls } from './ui/touch';
import { installStory } from './story';
import { state } from './core/state';
import { installDebug } from './core/debug';

const params = new URLSearchParams(location.search);
if (params.get('seed')) state.setSeedOverride(params.get('seed'));

ui.mount(document.getElementById('ui')!);
hud.mount();
mountTouchControls();
installStory();
app.start(document.getElementById('game')!);
if (params.has('debug')) installDebug();

void app.ready().then(() => app.title());
