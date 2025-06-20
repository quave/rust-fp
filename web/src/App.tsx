import { Container, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import { Transactions } from './components/Transactions';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    background: {
      default: '#111827',
    },
  },
});

function App() {
  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Container maxWidth={false} disableGutters>
        <Transactions />
      </Container>
    </ThemeProvider>
  );
}

export default App;
