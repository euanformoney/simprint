import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@/lib/tauri';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { toast } from 'sonner';
import {
  Server,
  PlugZap,
  Save,
  Loader2,
  CheckCircle2,
  XCircle,
} from 'lucide-react';

/**
 * 服务器配置信息
 */
interface ServerConfigDto {
  base_url: string;
  version: string;
  secret_key: string;
  is_user_configured: boolean;
}

/**
 * 连接测试结果
 */
interface TestResult {
  ok: boolean;
  message: string;
  http_status?: number | null;
}

/**
 * 配置向导属性
 */
interface ServerConfigWizardProps {
  /** 是否作为全屏向导（首次启动时）。false 时作为设置弹窗内容。 */
  fullscreen?: boolean;
  /** 保存成功后的回调 */
  onSaved?: () => void;
}

/**
 * 服务器配置向导组件
 *
 * 用于首次启动时配置服务器地址，也可在设置中复用。
 * 提供连接测试和保存功能。
 */
export function ServerConfigWizard({ fullscreen = true, onSaved }: ServerConfigWizardProps) {
  const [baseUrl, setBaseUrl] = useState('');
  const [version, setVersion] = useState('v1');
  const [secretKey, setSecretKey] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  // 加载当前生效的配置作为初始值
  useEffect(() => {
    invoke<ServerConfigDto>('get_server_config')
      .then((cfg) => {
        setBaseUrl(cfg.base_url);
        setVersion(cfg.version);
        setSecretKey(cfg.secret_key);
      })
      .catch((err) => {
        console.error('[ServerConfigWizard] 读取配置失败:', err);
        toast.error('读取当前配置失败');
      })
      .finally(() => setLoading(false));
  }, []);

  /** 连接测试 */
  const handleTest = useCallback(async () => {
    if (!baseUrl.trim()) {
      toast.error('请先填写服务器地址');
      return;
    }
    setTesting(true);
    setTestResult(null);
    try {
      const result = await invoke<TestResult>('test_server_connection', {
        baseUrl,
        version,
        secretKey,
      });
      setTestResult(result);
      if (result.ok) {
        toast.success(result.message);
      } else {
        toast.error(result.message);
      }
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : err?.message || '连接测试失败';
      setTestResult({ ok: false, message: msg });
      toast.error(msg);
    } finally {
      setTesting(false);
    }
  }, [baseUrl, version, secretKey]);

  /** 保存配置 */
  const handleSave = useCallback(async () => {
    if (!baseUrl.trim()) {
      toast.error('请填写服务器地址');
      return;
    }
    if (!version.trim()) {
      toast.error('请填写 API 版本');
      return;
    }
    if (!secretKey.trim()) {
      toast.error('请填写 secret key');
      return;
    }

    setSaving(true);
    try {
      await invoke('save_server_config', {
        baseUrl: baseUrl.trim(),
        version: version.trim(),
        secretKey: secretKey.trim(),
      });
      toast.success('配置已保存，即将重启应用');
      if (onSaved) {
        onSaved();
      } else {
        // 未提供回调时延迟重启，让用户看到成功提示
        setTimeout(() => {
          // 使用 tauri 的 relaunch 能力
          import('@tauri-apps/plugin-process')
            .then(({ relaunch }) => relaunch())
            .catch((e) => console.error('[ServerConfigWizard] 重启失败:', e));
        }, 1200);
      }
    } catch (err: any) {
      const msg = typeof err === 'string' ? err : err?.message || '保存失败';
      toast.error(msg);
    } finally {
      setSaving(false);
    }
  }, [baseUrl, version, secretKey, onSaved]);

  const wrapperCls = fullscreen
    ? 'flex min-h-screen w-full items-center justify-center bg-background p-4'
    : '';

  return (
    <div className={wrapperCls}>
      <div className="w-full max-w-md">
        <div className="text-center mb-8">
          <div className="mx-auto mb-4 flex size-14 items-center justify-center rounded-2xl bg-primary/10">
            <Server className="size-7 text-primary" />
          </div>
          <h1 className="text-2xl font-semibold tracking-tight">服务器配置</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            配置 Simprint 连接的服务器地址。保存后应用将重启生效。
          </p>
        </div>

        <div className="space-y-5 rounded-2xl border border-border/60 bg-card p-6">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <>
              <div className="space-y-2">
                <Label htmlFor="baseUrl">服务器地址</Label>
                <Input
                  id="baseUrl"
                  placeholder="https://your-server.com/api/"
                  value={baseUrl}
                  onChange={(e) => {
                    setBaseUrl(e.target.value);
                    setTestResult(null);
                  }}
                />
                <p className="text-xs text-muted-foreground">
                  例如：https://browser.aisub2api.top/api/
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="version">API 版本</Label>
                <Input
                  id="version"
                  placeholder="v1"
                  value={version}
                  onChange={(e) => {
                    setVersion(e.target.value);
                    setTestResult(null);
                  }}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="secretKey">Secret Key</Label>
                <Input
                  id="secretKey"
                  type="password"
                  placeholder="服务器密钥"
                  value={secretKey}
                  onChange={(e) => {
                    setSecretKey(e.target.value);
                    setTestResult(null);
                  }}
                />
                <p className="text-xs text-muted-foreground">
                  与服务端 install-server.sh 中的默认 secret 一致
                </p>
              </div>

              {testResult && (
                <div
                  className={`flex items-start gap-2 rounded-lg border p-3 text-sm ${
                    testResult.ok
                      ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700'
                      : 'border-destructive/30 bg-destructive/10 text-destructive'
                  }`}
                >
                  {testResult.ok ? (
                    <CheckCircle2 className="mt-0.5 size-4 shrink-0" />
                  ) : (
                    <XCircle className="mt-0.5 size-4 shrink-0" />
                  )}
                  <span className="break-all">{testResult.message}</span>
                </div>
              )}

              <div className="flex gap-3 pt-2">
                <Button
                  variant="outline"
                  className="flex-1"
                  onClick={handleTest}
                  disabled={testing || !baseUrl.trim()}
                >
                  {testing ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <PlugZap className="size-4" />
                  )}
                  连接测试
                </Button>
                <Button
                  className="flex-1"
                  onClick={handleSave}
                  disabled={saving || !baseUrl.trim() || !secretKey.trim()}
                >
                  {saving ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <Save className="size-4" />
                  )}
                  保存并重启
                </Button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
