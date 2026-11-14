import { AllGamesScreen } from '@/features/library/AllGamesScreen';
import { HomeScreen } from '@/features/library/HomeScreen';
import { useNavigation } from './navigation/store';

/**
 * Pilha de telas (arch 3.1): so a camada do topo recebe foco e anel; as de baixo ficam montadas
 * (o foco volta ao item que abriu a camada, handoff 4).
 */
export function ScreenStack({ headerExtras }: { headerExtras?: React.ReactNode }) {
  const stack = useNavigation((s) => s.stack);
  return (
    <>
      {stack.map((entry, index) => {
        const active = index === stack.length - 1;
        switch (entry.kind) {
          case 'home':
            return <HomeScreen key={entry.key} active={active} headerExtras={headerExtras} />;
          case 'allGames':
            return <AllGamesScreen key={entry.key} active={active} headerExtras={headerExtras} />;
          default:
            return null;
        }
      })}
    </>
  );
}
