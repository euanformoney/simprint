import { Server } from 'lucide-react';
import { ServerConfigWizard } from '@/components/server-config-wizard';
import { SettingCard } from './setting-card';

/**
 * 服务器配置面板
 *
 * 在系统设置中提供服务器地址、secret key 的配置入口。
 * 复用 ServerConfigWizard 组件，以非全屏模式嵌入。
 */
export function ServerPanel() {
  return (
    <SettingCard title="服务器配置" icon={Server}>
      <ServerConfigWizard fullscreen={false} />
    </SettingCard>
  );
}
