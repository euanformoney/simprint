// 确保扩展点在所有插件导入之前注册
import '../../extension-points';

import React, { useEffect, useState } from 'react';
import { BrowserRouter } from 'react-router';
import { pluginRegistry } from '@slotkitjs/core';
import { useAuthStore } from '../../../../plugins/services/store/src';
import { SettingsDialog } from '../../../../plugins/pages/system-settings/src';
import { SettingsBootstrap } from '../../../../plugins/services/store/src';
import { ThemeProvider } from '@/components/theme-provider';
import { I18nProvider } from '@/components/i18n-provider';
import { commonResources } from '@/i18n/resources/common';
import { Toaster } from '@/components/ui/sonner';
import { ServerConfigWizard } from '@/components/server-config-wizard';
import { invoke } from '@/lib/tauri';
import { AppRoutes } from '../AppRoutes';
import { SplashscreenRenderer } from '../SplashscreenRenderer';
import { SyncerRenderer } from '../SyncerRenderer';
import { useDisableDevTools } from '@/hooks/use-disable-dev-tools';
import { SessionLockOverlay } from '../session-lock-overlay';
import { useSessionLock } from '../../hooks/use-session-lock';

interface AppLayoutProps {
  /**
   * 布局模式：'main' 渲染主应用布局，'splashscreen' 渲染启动屏幕，'syncer' 渲染同步器窗口
   */
  mode?: 'main' | 'splashscreen' | 'syncer';
}

export const AppLayout: React.FC<AppLayoutProps> = ({ mode = 'main' }) => {
  const [windowManagerComponent, setWindowManagerComponent] = useState<React.ComponentType | null>(
    null
  );
  const [serverConfigChecked, setServerConfigChecked] = useState(false);
  const { initAuth, isAuthenticated } = useAuthStore();
  useDisableDevTools();
  const sessionLock = useSessionLock(mode === 'main');

  // 检查服务器配置状态（仅在 main 模式下）
  useEffect(() => {
    if (mode === 'main') {
      invoke<{ is_user_configured: boolean }>('get_server_config')
        .then((cfg) => {
          setServerConfigChecked(cfg.is_user_configured);
        })
        .catch((error) => {
          console.error('[AppLayout] 读取服务器配置失败:', error);
          // 读取失败时不阻塞，按已配置处理
          setServerConfigChecked(true);
        });
    } else {
      setServerConfigChecked(true);
    }
  }, [mode]);

  // 初始化认证状态（仅在 main 模式下）
  useEffect(() => {
    if (mode === 'main') {
      initAuth().catch((error) => {
        console.error('[AppLayout] 初始化认证状态失败:', error);
      });
    }
  }, [mode, initAuth]);

  // 监听插件注册，确保 window-manager 插件加载后被渲染
  useEffect(() => {
    const checkWindowManager = () => {
      const windowManagerPlugin = pluginRegistry.getPlugin('window-manager');
      if (windowManagerPlugin?.component) {
        setWindowManagerComponent(() => windowManagerPlugin.component);
        console.log('[AppLayout] WindowManager plugin loaded');
      }
    };

    // 立即检查一次
    checkWindowManager();

    // 订阅插件注册事件
    const unsubscribe = pluginRegistry.subscribe((event) => {
      if (event.type === 'register' && event.plugin?.id === 'window-manager') {
        checkWindowManager();
      }
    });

    return () => {
      unsubscribe();
    };
  }, []);

  // 如果是 splashscreen 模式，渲染启动屏幕
  if (mode === 'splashscreen') {
    return <SplashscreenRenderer />;
  }

  // 如果是 syncer 模式，渲染同步器窗口
  if (mode === 'syncer') {
    return (
      <ThemeProvider defaultTheme="system" storageKey="simprint-ui-theme">
        <SyncerRenderer />
      </ThemeProvider>
    );
  }

  // 首次启动：未配置服务器时渲染配置向导，阻止进入主界面
  if (mode === 'main' && !serverConfigChecked) {
    return (
      <ThemeProvider defaultTheme="system" storageKey="simprint-ui-theme">
        <ServerConfigWizard />
        <Toaster />
      </ThemeProvider>
    );
  }

  // 默认渲染主应用布局
  return (
    <SettingsBootstrap>
      <ThemeProvider defaultTheme="system" storageKey="simprint-ui-theme">
        <I18nProvider resources={{ common: commonResources }}>
        <BrowserRouter>
          {windowManagerComponent && React.createElement(windowManagerComponent)}
          <AppRoutes />
          <SettingsDialog />
          <SessionLockOverlay
            open={mode === 'main' && isAuthenticated && sessionLock.isLocked}
            unlocking={sessionLock.unlocking}
            error={sessionLock.error}
            onUnlock={sessionLock.unlock}
          />
          <Toaster />
        </BrowserRouter>
        </I18nProvider>
      </ThemeProvider>
    </SettingsBootstrap>
  );
};
