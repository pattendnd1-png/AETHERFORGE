import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PluginManager } from './PluginManager';
import type { PluginDescriptor } from '../plugins/types';

function plugin(overrides: Partial<PluginDescriptor> = {}): PluginDescriptor {
  return {
    uuid: 'com.example.one',
    name: 'Plugin One',
    version: '1.0.0',
    author: 'Example',
    description: 'Example plugin',
    sourceKind: 'elgato',
    root: '/tmp/plugin-one',
    enabled: true,
    active: false,
    processState: 'inactive',
    compatibility: 'node',
    runtimeKind: 'node',
    lastError: null,
    sdkVersion: 2,
    minimumSoftwareVersion: '7.6',
    propertyInspectorPath: null,
    actions: [],
    profiles: [],
    ...overrides,
  };
}

function props() {
  return {
    plugins: [
      plugin(),
      plugin({ uuid: 'com.example.two', name: 'Plugin Two', enabled: false }),
    ],
    iconPacks: [{
      id: 'com.example.icons',
      name: 'Example Icons',
      version: '1.0.0',
      author: 'Example',
      description: 'Example icon pack',
      root: '/tmp/example-icons',
      active: false,
      itemCount: 24,
    }],
    hostStatus: { protocolVersion: '2.0.47', streamDeckCompatibilityTarget: '7.6', websocketHost: '127.0.0.1', installed: 2, active: 0, enabled: 1 },
    marketplaceItems: [
      { name: 'downloaded.streamDeckPlugin', path: '/tmp/downloaded.streamDeckPlugin', kind: 'plugin' },
      { name: 'icons.streamDeckIconPack', path: '/tmp/icons.streamDeckIconPack', kind: 'icon_pack' },
      { name: 'profile.streamDeckProfile', path: '/tmp/profile.streamDeckProfile', kind: 'profile' },
    ],
    onScan: vi.fn().mockResolvedValue(undefined),
    onOpenMarketplace: vi.fn().mockResolvedValue(undefined),
    onInstallPackage: vi.fn().mockResolvedValue(undefined),
    onSetEnabled: vi.fn().mockResolvedValue(undefined),
    onSetActive: vi.fn().mockResolvedValue(undefined),
    onRestart: vi.fn().mockResolvedValue(undefined),
    onRemove: vi.fn().mockResolvedValue(undefined),
    onSetIconPackActive: vi.fn().mockResolvedValue(undefined),
    onRemoveIconPack: vi.fn().mockResolvedValue(undefined),
    onImportBundledProfile: vi.fn().mockResolvedValue(undefined),
  };
}

describe('PluginManager selection and activation', () => {
  it('selects a disabled plugin and activates it directly', async () => {
    const input = props();
    render(<PluginManager {...input} />);

    fireEvent.click(screen.getByRole('button', { name: /Plugin Two/i }));
    const activate = screen.getByRole('button', { name: 'Activate Plugin' });
    expect(activate).toBeEnabled();
    fireEvent.click(activate);

    await waitFor(() => expect(input.onSetActive).toHaveBeenCalledWith('com.example.two', true));
  });

  it('selects an installed icon pack and activates it', async () => {
    const input = props();
    render(<PluginManager {...input} />);

    fireEvent.click(screen.getByRole('button', { name: /Example Icons/i }));
    fireEvent.click(screen.getByRole('button', { name: 'Activate Pack' }));

    await waitFor(() => expect(input.onSetIconPackActive).toHaveBeenCalledWith('com.example.icons', true));
  });

  it('selects downloaded plugin and pack packages and exposes install-and-activate', async () => {
    const input = props();
    render(<PluginManager {...input} />);

    fireEvent.click(screen.getByRole('button', { name: /icons\.streamDeckIconPack/i }));
    fireEvent.click(screen.getByRole('button', { name: 'Install & Activate' }));
    await waitFor(() => expect(input.onInstallPackage).toHaveBeenCalledWith('/tmp/icons.streamDeckIconPack'));

    fireEvent.click(screen.getByRole('button', { name: /profile\.streamDeckProfile/i }));
    expect(screen.getByRole('button', { name: 'Import & Activate Profile' })).toBeEnabled();
  });
});
