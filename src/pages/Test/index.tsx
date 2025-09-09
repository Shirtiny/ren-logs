import { useState, useEffect, useRef } from 'react';
import Button from '@/components/Button';
import './index.scss';

interface RequestInit {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
}

interface ApiResponse {
  code: number;
  [key: string]: any;
}

interface WebSocketData {
  code: number;
  user: any;
}

const Component = () => {
  const [response, setResponse] = useState<string>('');
  const [loading, setLoading] = useState<string>('');
  const [wsConnected, setWsConnected] = useState(false);
  const [wsData, setWsData] = useState<string>('');
  const [pauseValue, setPauseValue] = useState<boolean>(false);
  const [settings, setSettings] = useState({
    auto_clear_on_server_change: false,
    auto_clear_on_timeout: false,
    only_record_elite_dummy: false,
  });
  const [skillUid, setSkillUid] = useState<string>('1');
  const [historyTimestamp, setHistoryTimestamp] = useState<string>('');

  const wsRef = useRef<WebSocket | null>(null);
  const BASE_URL = 'http://127.0.0.1:8989';

  // WebSocket连接
  useEffect(() => {
    connectWebSocket();
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  const connectWebSocket = () => {
    try {
      const ws = new WebSocket(`ws://127.0.0.1:8989/ws`);
      wsRef.current = ws;

      ws.onopen = () => {
        setWsConnected(true);
        setWsData('WebSocket connected successfully');
      };

      ws.onmessage = (event) => {
        try {
          const data: WebSocketData = JSON.parse(event.data);
          setWsData(JSON.stringify(data, null, 2));
        } catch (e) {
          setWsData(`Received: ${event.data}`);
        }
      };

      ws.onclose = () => {
        setWsConnected(false);
        setWsData('WebSocket disconnected');
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setWsData('WebSocket error occurred');
      };
    } catch (e) {
      setWsData('Failed to connect WebSocket');
    }
  };

  const makeRequest = async (method: 'GET' | 'POST', endpoint: string, body?: any) => {
    setLoading(endpoint);
    try {
      const options: RequestInit = {
        method,
        headers: {
          'Content-Type': 'application/json',
        },
      };

      if (body) {
        options.body = JSON.stringify(body);
      }

      const res = await fetch(`${BASE_URL}${endpoint}`, options);
      const data: ApiResponse = await res.json();
      setResponse(JSON.stringify(data, null, 2));
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      setLoading('');
    }
  };

  const apiTests = [
    {
      name: 'Get User Data',
      endpoint: '/api/data',
      method: 'GET' as const,
      description: '获取所有用户数据'
    },
    {
      name: 'Get Enemy Data',
      endpoint: '/api/enemies',
      method: 'GET' as const,
      description: '获取所有敌人数据'
    },
    {
      name: 'Clear Data',
      endpoint: '/api/clear',
      method: 'GET' as const,
      description: '清除所有统计数据'
    },
    {
      name: 'Get Pause Status',
      endpoint: '/api/pause',
      method: 'GET' as const,
      description: '获取暂停状态'
    },
    {
      name: 'Get Settings',
      endpoint: '/api/settings',
      method: 'GET' as const,
      description: '获取当前设置'
    },
    {
      name: 'Health Check',
      endpoint: '/api/health',
      method: 'GET' as const,
      description: '健康检查'
    },
    {
      name: 'List History',
      endpoint: '/api/history/list',
      method: 'GET' as const,
      description: '列出历史快照',
    },
  ];

  return (
    <div className="page page-test">
      <div className="test-container">
        <div className="api-panel">
          <h2>API 测试面板</h2>

          {/* 基础API测试 */}
          <div className="api-section">
            <h3>基础接口</h3>
            {apiTests.map((api) => (
              <div key={api.endpoint} className="api-item">
                <Button
                  onClick={() => makeRequest(api.method, api.endpoint)}
                  disabled={loading === api.endpoint}
                  className="api-button"
                >
                  {loading === api.endpoint ? 'Loading...' : api.name}
                </Button>
                <span className="api-desc">{api.description}</span>
              </div>
            ))}
          </div>

          {/* POST请求 */}
          <div className="api-section">
            <h3>POST 接口</h3>

            {/* 设置暂停状态 */}
            <div className="api-item">
              <div className="input-group">
                <label>
                  <input
                    type="checkbox"
                    checked={pauseValue}
                    onChange={(e) => setPauseValue(e.target.checked)}
                  />
                  暂停统计
                </label>
                <Button
                  onClick={() => makeRequest('POST', '/api/pause', { paused: pauseValue })}
                  disabled={loading === '/api/pause'}
                  className="api-button"
                >
                  {loading === '/api/pause' ? 'Loading...' : 'Set Pause Status'}
                </Button>
              </div>
              <span className="api-desc">设置统计暂停状态</span>
            </div>

            {/* 更新设置 */}
            <div className="api-item">
              <div className="input-group">
                <label>
                  <input
                    type="checkbox"
                    checked={settings.auto_clear_on_server_change}
                    onChange={(e) => setSettings(prev => ({
                      ...prev,
                      auto_clear_on_server_change: e.target.checked
                    }))}
                  />
                  服务器切换时自动清除
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={settings.auto_clear_on_timeout}
                    onChange={(e) => setSettings(prev => ({
                      ...prev,
                      auto_clear_on_timeout: e.target.checked
                    }))}
                  />
                  超时自动清除
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={settings.only_record_elite_dummy}
                    onChange={(e) => setSettings(prev => ({
                      ...prev,
                      only_record_elite_dummy: e.target.checked
                    }))}
                  />
                  只记录精英假人
                </label>
                <Button
                  onClick={() => makeRequest('POST', '/api/settings', settings)}
                  disabled={loading === '/api/settings'}
                  className="api-button"
                >
                  {loading === '/api/settings' ? 'Loading...' : 'Update Settings'}
                </Button>
              </div>
              <span className="api-desc">更新系统设置</span>
            </div>
          </div>

          {/* 带参数的GET请求 */}
          <div className="api-section">
            <h3>带参数接口</h3>

            {/* 获取用户技能数据 */}
            <div className="api-item">
              <div className="input-group">
                <input
                  type="text"
                  placeholder="用户UID"
                  value={skillUid}
                  onChange={(e) => setSkillUid(e.target.value)}
                  className="input-field"
                />
                <Button
                  onClick={() => makeRequest('GET', `/api/skill/${skillUid}`)}
                  disabled={loading === `/api/skill/${skillUid}`}
                  className="api-button"
                >
                  {loading === `/api/skill/${skillUid}` ? 'Loading...' : 'Get User Skill Data'}
                </Button>
              </div>
              <span className="api-desc">获取指定用户的技能数据</span>
            </div>

            {/* 获取历史快照 */}
            <div className="api-item">
              <div className="input-group">
                <input
                  type="text"
                  placeholder="时间戳"
                  value={historyTimestamp}
                  onChange={(e) => setHistoryTimestamp(e.target.value)}
                  className="input-field"
                />
                <Button
                  onClick={() => makeRequest('GET', `/api/history/${historyTimestamp}`)}
                  disabled={loading === `/api/history/${historyTimestamp}`}
                  className="api-button"
                >
                  {loading === `/api/history/${historyTimestamp}` ? 'Loading...' : 'Get History Snapshot'}
                </Button>
              </div>
              <span className="api-desc">获取指定时间戳的历史快照</span>
            </div>
          </div>

          {/* WebSocket状态 */}
          <div className="api-section">
            <h3>WebSocket 实时数据</h3>
            <div className="ws-status">
              <span className={`status-indicator ${wsConnected ? 'connected' : 'disconnected'}`}>
                {wsConnected ? '已连接' : '未连接'}
              </span>
              <Button onClick={connectWebSocket} disabled={wsConnected}>
                重新连接
              </Button>
            </div>
          </div>
        </div>

        <div className="result-panel">
          <h2>响应结果</h2>
          <div className="result-content">
            <pre>{response || '点击API按钮查看结果'}</pre>
          </div>

          <h2>实时数据 (WebSocket)</h2>
          <div className="result-content">
            <pre>{wsData || '等待WebSocket数据...'}</pre>
          </div>
        </div>
      </div>
    </div>
  );
};

export { Component };
