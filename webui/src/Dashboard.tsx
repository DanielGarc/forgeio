import { useEffect, useState } from 'react'

export default function Dashboard() {
  const [health, setHealth] = useState<string>('')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    fetch('/api/health')
      .then(res => {
        if (!res.ok) throw new Error('Failed to fetch health status')
        return res.text()
      })
      .then(setHealth)
      .catch(err => setError(err.message))
  }, [])

  return (
    <div>
      <h1>Admin Dashboard</h1>
      {error && <p className="error">{error}</p>}
      {health && <p>{health}</p>}
    </div>
  )
}
