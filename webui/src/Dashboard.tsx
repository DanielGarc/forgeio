import { useEffect, useState } from 'react'

interface Stats {
  uptime_seconds: number
  tag_count: number
  driver_count: number
}

export default function Dashboard() {
  const [health, setHealth] = useState<string>('')
  const [stats, setStats] = useState<Stats | null>(null)
  const [error, setError] = useState<string | null>(null)

  const headers = { Authorization: 'Basic ' + btoa('admin:admin') }

  useEffect(() => {
    fetch('/api/health', { headers })
      .then(res => {
        if (!res.ok) throw new Error('Failed to fetch health status')
        return res.text()
      })
      .then(setHealth)
      .catch(err => setError(err.message))

    fetch('/api/stats', { headers })
      .then(res => {
        if (!res.ok) throw new Error('Failed to fetch stats')
        return res.json()
      })
      .then(data => setStats(data as Stats))
      .catch(err => setError(err.message))
  }, [])

  return (
    <div>
      <h1>Admin Dashboard</h1>
      {error && <p className="error">{error}</p>}
      {health && <p>{health}</p>}
      {stats && (
        <ul>
          <li>Uptime: {stats.uptime_seconds}s</li>
          <li>Drivers: {stats.driver_count}</li>
          <li>Tags: {stats.tag_count}</li>
        </ul>
      )}
    </div>
  )
}
