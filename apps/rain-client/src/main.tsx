import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import './theme.css';

const rootElement = document.getElementById('root') || document.getElementById('app');
if (rootElement) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}

