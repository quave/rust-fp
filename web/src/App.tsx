import { Container, Typography, Box } from '@mui/material';
import { Orders } from './components/Orders';

function App() {
  return (
    <Container maxWidth="lg">
      <Box sx={{ my: 4 }}>
        <Typography variant="h3" component="h1" gutterBottom>
          Welcome to Frida Web
        </Typography>
        <Orders />
      </Box>
    </Container>
  );
}

export default App;
