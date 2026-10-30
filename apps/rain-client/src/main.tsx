import React from 'react';
import ReactDOM from 'react-dom/client';
import {AdminBoundary} from './components/AdminBoundary';
import { App } from './App';
import './theme.css';

const rootElement = document.getElementById('root') || document.getElementById('app');
if (rootElement) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <AdminBoundary><App /></AdminBoundary>
    </React.StrictMode>
  );
}

