import { lazy, Suspense } from 'react';
import { AllGamesScreen } from '@/features/library/AllGamesScreen';
import { GameOptions } from '@/features/library/GameOptions';
import { HideConfirmDialog } from '@/features/library/HideConfirmDialog';
import { ExitDialog } from '@/features/quickmenu/ExitDialog';
import { PowerDialog } from '@/features/quickmenu/PowerDialog';
import { QuickMenu } from '@/features/quickmenu/QuickMenu';
import { HomeScreen } from '@/features/library/HomeScreen';
import { useLibrary } from '@/features/library/store';
import { CloseConfirmDialog } from '@/features/session/CloseConfirmDialog';
import { LaunchFlow, useLaunchFlowEntry } from '@/features/session/LaunchFlow';
import { SessionPanel } from '@/features/session/SessionPanel';
import { Toast } from '@/shared/ui/Toast';
import { useNavigation } from './navigation/store';

const RenameDialog = lazy(() => import('@/features/library/RenameDialog'));
// Configuracoes e navegador de pastas: chunks proprios, so baixam quando a tela abre (arch 3.4).
const SettingsScreen = lazy(() => import('@/features/settings/SettingsScreen'));
const RestoreDialog = lazy(() => import('@/features/settings/RestoreDialog'));
const FolderBrowser = lazy(() => import('@/features/folders/FolderBrowser'));

/**
 * Pilha de telas (arch 3.1): so a camada do topo recebe foco e anel; as de baixo ficam montadas
 * (o foco volta ao item que abriu a camada, handoff 4).
 */
export function ScreenStack({ headerExtras }: { headerExtras?: React.ReactNode }) {
  const stack = useNavigation((s) => s.stack);
  useLaunchFlowEntry();
  const notice = useLibrary((s) => s.notice);
  const clearNotice = useLibrary((s) => s.clearNotice);
  return (
    <>
      {stack.map((entry, index) => {
        const active = index === stack.length - 1;
        const parentKey = stack[index - 1]?.key;
        switch (entry.kind) {
          case 'home':
            return <HomeScreen key={entry.key} active={active} headerExtras={headerExtras} />;
          case 'allGames':
            return <AllGamesScreen key={entry.key} active={active} headerExtras={headerExtras} />;
          case 'gameOptions':
            return <GameOptions key={entry.key} active={active} entry={entry} />;
          case 'confirmDialog':
            switch (entry.params?.confirm) {
              case 'hide':
                return <HideConfirmDialog key={entry.key} active={active} entry={entry} parentKey={parentKey} />;
              case 'close':
                return <CloseConfirmDialog key={entry.key} active={active} entry={entry} />;
              case 'restore':
                return (
                  <Suspense key={entry.key} fallback={null}>
                    <RestoreDialog active={active} entry={entry} />
                  </Suspense>
                );
              case 'power':
                return <PowerDialog key={entry.key} active={active} entry={entry} />;
              case 'exit':
                return <ExitDialog key={entry.key} active={active} entry={entry} />;
              default:
                return null;
            }
          case 'settings':
            return (
              <Suspense key={entry.key} fallback={null}>
                <SettingsScreen active={active} entry={entry} headerExtras={headerExtras} />
              </Suspense>
            );
          case 'folderBrowser':
            return (
              <Suspense key={entry.key} fallback={null}>
                <FolderBrowser active={active} entry={entry} headerExtras={headerExtras} />
              </Suspense>
            );
          case 'quickMenu':
            return <QuickMenu key={entry.key} active={active} entry={entry} />;
          case 'sessionPanel':
            return <SessionPanel key={entry.key} active={active} entry={entry} />;
          case 'renameDialog':
            return (
              <Suspense key={entry.key} fallback={null}>
                <RenameDialog active={active} entry={entry} parentKey={parentKey} />
              </Suspense>
            );
          case 'launchFlow':
            return <LaunchFlow key={entry.key} active={active} />;
          default:
            return null;
        }
      })}
      <Toast message={notice} onDone={clearNotice} />
    </>
  );
}
