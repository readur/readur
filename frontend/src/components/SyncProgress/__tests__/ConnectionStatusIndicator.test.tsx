import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConnectionStatusIndicator } from '../ConnectionStatusIndicator';

describe('ConnectionStatusIndicator', () => {
  test('should display connecting status', () => {
    render(<ConnectionStatusIndicator connectionStatus="connecting" />);
    expect(screen.getByText('Connecting...')).toBeInTheDocument();
  });

  test('should display reconnecting status', () => {
    render(<ConnectionStatusIndicator connectionStatus="reconnecting" />);
    expect(screen.getByText('Reconnecting...')).toBeInTheDocument();
  });

  test('should display connected status when not active', () => {
    render(<ConnectionStatusIndicator connectionStatus="connected" isActive={false} />);
    expect(screen.getByText('Connected')).toBeInTheDocument();
  });

  test('should display live status when active', () => {
    render(<ConnectionStatusIndicator connectionStatus="connected" isActive={true} />);
    expect(screen.getByText('Live')).toBeInTheDocument();
  });

  test('should display disconnected status', () => {
    render(<ConnectionStatusIndicator connectionStatus="disconnected" />);
    expect(screen.getByText('Disconnected')).toBeInTheDocument();
  });

  test('should display error status', () => {
    render(<ConnectionStatusIndicator connectionStatus="error" />);
    expect(screen.getByText('Disconnected')).toBeInTheDocument();
  });

  test('should display failed status', () => {
    render(<ConnectionStatusIndicator connectionStatus="failed" />);
    expect(screen.getByText('Connection Failed')).toBeInTheDocument();
  });

  test('should show reconnect button on failure', () => {
    const onReconnect = vi.fn();
    render(
      <ConnectionStatusIndicator 
        connectionStatus="failed" 
        onReconnect={onReconnect}
      />
    );
    
    const reconnectButton = screen.getByRole('button', { name: /reconnect/i });
    expect(reconnectButton).toBeInTheDocument();
    
    fireEvent.click(reconnectButton);
    expect(onReconnect).toHaveBeenCalled();
  });

  test('should show reconnect button on error', () => {
    const onReconnect = vi.fn();
    render(
      <ConnectionStatusIndicator 
        connectionStatus="error" 
        onReconnect={onReconnect}
      />
    );
    
    expect(screen.getByRole('button', { name: /reconnect/i })).toBeInTheDocument();
  });

  test('should not show reconnect button when connected', () => {
    const onReconnect = vi.fn();
    render(
      <ConnectionStatusIndicator 
        connectionStatus="connected" 
        onReconnect={onReconnect}
      />
    );
    
    expect(screen.queryByRole('button', { name: /reconnect/i })).not.toBeInTheDocument();
  });
});