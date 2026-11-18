import { FaDownload, FaRocket, FaTag, FaTrash } from 'react-icons/fa'
import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { releasesApi } from '../api/releases'
import type { ReleaseResponse } from '../bindings/ReleaseResponse'
import { Markdown } from '../components/Markdown'
import { releaseRequested } from '../store/releasesSlice'
import { useAppDispatch, useAppSelector } from '../store/hooks'
import { card, input, label, primaryButton } from '../ui'
import { canWriteRole, useRepoParams, useRepoRole } from './RepoLayout'

function shortSha(sha: string): string {
  return sha ? sha.slice(0, 7) : 'tag missing'
}

/** Releases, their git tags, and the source archive for each tag. */
export function Releases() {
  const dispatch = useAppDispatch()
  const { owner, name } = useRepoParams()
  const role = useRepoRole()
  const canWrite = canWriteRole(role)
  const { releases, tags, loading, error, current } = useAppSelector(
    (state) => state.releases,
  )
  const [tagName, setTagName] = useState('')
  const [target, setTarget] = useState('')
  const [title, setTitle] = useState('')
  const [body, setBody] = useState('')
  const [isPrerelease, setIsPrerelease] = useState(false)

  useEffect(() => {
    const key = `${owner}/${name}`
    dispatch(
      releaseRequested({ part: 'list', key: `releases:${key}`, owner, name }),
    )
    dispatch(releaseRequested({ part: 'tags', key: `tags:${key}`, owner, name }))
  }, [dispatch, owner, name])

  const releasedTags = new Set(releases.map((release) => release.tag_name))
  const looseTags = tags.filter((tag) => !releasedTags.has(tag.name))

  function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    dispatch(
      releaseRequested({
        part: 'create',
        key: `create:${owner}/${name}:${Date.now()}`,
        owner,
        name,
        params: {
          tag_name: tagName,
          target: target || 'HEAD',
          title,
          body: body || null,
          is_prerelease: isPrerelease,
        },
      }),
    )
    setTagName('')
    setTarget('')
    setTitle('')
    setBody('')
    setIsPrerelease(false)
  }

  return (
    <div className="space-y-6">
      {canWrite && (
        <form onSubmit={handleCreate} className={`${card} space-y-4 p-5`}>
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <FaRocket className="text-blue-600" /> New release
          </h2>
          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <label htmlFor="release-tag" className={label}>Tag</label>
              <input
                id="release-tag"
                required
                value={tagName}
                onChange={(event) => setTagName(event.target.value)}
                placeholder="v1.0.0"
                className={`${input} mt-1`}
              />
            </div>
            <div>
              <label htmlFor="release-target" className={label}>
                Target branch or commit
              </label>
              <input
                id="release-target"
                value={target}
                onChange={(event) => setTarget(event.target.value)}
                placeholder="HEAD"
                className={`${input} mt-1`}
              />
            </div>
          </div>
          <div>
            <label htmlFor="release-title" className={label}>Title</label>
            <input
              id="release-title"
              required
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="Version 1.0.0"
              className={`${input} mt-1`}
            />
          </div>
          <div>
            <label htmlFor="release-body" className={label}>Release notes</label>
            <textarea
              id="release-body"
              rows={4}
              value={body}
              onChange={(event) => setBody(event.target.value)}
              className={`${input} mt-1`}
            />
          </div>
          <label className="flex items-center gap-2 text-sm text-slate-700">
            <input
              type="checkbox"
              checked={isPrerelease}
              onChange={(event) => setIsPrerelease(event.target.checked)}
            />
            Set as a pre-release
          </label>
          <button type="submit" disabled={loading.create} className={primaryButton}>
            <FaRocket /> {loading.create ? 'Publishing…' : 'Publish release'}
          </button>
          {error.create && (
            <p role="alert" className="text-sm text-red-700">{error.create}</p>
          )}
        </form>
      )}

      <section className={`${card} overflow-hidden`}>
        <div className="border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="text-lg font-semibold">Releases</h2>
        </div>
        {loading.list && <p className="p-5 text-sm text-slate-500">Loading…</p>}
        {error.list && (
          <p role="alert" className="p-5 text-sm text-red-700">{error.list}</p>
        )}
        {!loading.list && releases.length === 0 && (
          <p className="p-5 text-sm text-slate-500">No releases yet.</p>
        )}
        <ul className="divide-y divide-slate-100">
          {releases.map((release: ReleaseResponse) => (
            <li key={release.id} className="space-y-2 p-5">
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="text-lg font-medium">{release.title}</h3>
                {release.is_prerelease && (
                  <span className="rounded-full bg-amber-100 px-2 py-0.5 text-xs text-amber-800">
                    Pre-release
                  </span>
                )}
                <span className="inline-flex items-center gap-1 rounded-full bg-blue-50 px-2 py-0.5 font-mono text-xs text-blue-700">
                  <FaTag /> {release.tag_name}
                </span>
              </div>
              <p className="text-xs text-slate-500">
                {release.author} released this {release.published_at.slice(0, 10)} ·{' '}
                <span className="font-mono">{shortSha(release.commit_sha)}</span>
              </p>
              {release.body && <Markdown source={release.body} />}
              <div className="flex flex-wrap items-center gap-3">
                <a
                  href={releasesApi.archiveUrl(owner, name, release.tag_name)}
                  className="inline-flex items-center gap-1 text-sm text-blue-600 hover:underline"
                >
                  <FaDownload /> Download source (.tar.gz)
                </a>
                {canWrite && (
                  <>
                    <button
                      type="button"
                      className="text-sm text-blue-600 hover:underline"
                      onClick={() =>
                        dispatch(
                          releaseRequested({
                            part: 'update',
                            key: `update:${release.id}:${Date.now()}`,
                            owner,
                            name,
                            id: release.id,
                            params: {
                              title: release.title,
                              body: release.body,
                              is_prerelease: !release.is_prerelease,
                            },
                          }),
                        )
                      }
                    >
                      {release.is_prerelease ? 'Mark as stable' : 'Mark as pre-release'}
                    </button>
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 text-sm text-red-600 hover:underline"
                      onClick={() =>
                        dispatch(
                          releaseRequested({
                            part: 'remove',
                            key: `remove:${release.id}:${Date.now()}`,
                            owner,
                            name,
                            id: release.id,
                          }),
                        )
                      }
                    >
                      <FaTrash /> Delete
                    </button>
                  </>
                )}
              </div>
              {current?.id === release.id && error.update && (
                <p role="alert" className="text-sm text-red-700">{error.update}</p>
              )}
            </li>
          ))}
        </ul>
      </section>

      <section className={`${card} overflow-hidden`}>
        <div className="border-b border-slate-200 bg-slate-50 px-5 py-4">
          <h2 className="text-lg font-semibold">Tags</h2>
        </div>
        {looseTags.length === 0 && (
          <p className="p-5 text-sm text-slate-500">No tags without a release.</p>
        )}
        <ul className="divide-y divide-slate-100">
          {looseTags.map((tag) => (
            <li key={tag.name} className="flex items-center justify-between px-5 py-3">
              <div>
                <p className="font-mono text-sm">{tag.name}</p>
                <p className="text-xs text-slate-500">{shortSha(tag.sha)}</p>
              </div>
              {canWrite && (
                <button
                  type="button"
                  title={`Delete ${tag.name}`}
                  className="rounded p-1 text-red-600 hover:bg-red-50"
                  onClick={() =>
                    dispatch(
                      releaseRequested({
                        part: 'removeTag',
                        key: `remove-tag:${tag.name}:${Date.now()}`,
                        owner,
                        name,
                        tag: tag.name,
                      }),
                    )
                  }
                >
                  <FaTrash />
                </button>
              )}
            </li>
          ))}
        </ul>
      </section>
    </div>
  )
}
