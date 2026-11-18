import type { PayloadAction } from '@reduxjs/toolkit'
import { all, call, put, takeEvery, takeLatest } from 'redux-saga/effects'
import type { SagaIterator } from 'redux-saga'
import { ApiClientError } from '../api/client'
import { collaboratorsApi } from '../api/collaborators'
import { issuesApi } from '../api/issues'
import { pullsApi } from '../api/pulls'
import { repositoriesApi } from '../api/repositories'
import { socialApi } from '../api/social'
import { tokensApi } from '../api/tokens'
import type { AddCommentParams } from '../bindings/AddCommentParams'
import type { AddReviewCommentParams } from '../bindings/AddReviewCommentParams'
import type { CreateIssueParams } from '../bindings/CreateIssueParams'
import type { CreatePullRequestParams } from '../bindings/CreatePullRequestParams'
import type { CreateLabelParams } from '../bindings/CreateLabelParams'
import type { CreateRepoParams } from '../bindings/CreateRepoParams'
import type { CreateTokenParams } from '../bindings/CreateTokenParams'
import type { RepoResponse } from '../bindings/RepoResponse'
import {
  browseFailed,
  browseRequested,
  browseSucceeded,
  type BrowseRequest,
} from './browseSlice'
import {
  collaboratorFailed,
  collaboratorRequested,
  collaboratorSucceeded,
  type CollaboratorRequest,
} from './collaboratorsSlice'
import {
  issueFailed,
  issueRequested,
  issueSucceeded,
  type IssueRequest,
} from './issuesSlice'
import {
  pullFailed,
  pullRequested,
  pullSucceeded,
  type PullRequest as PullRequestAction,
} from './pullsSlice'
import {
  socialFailed,
  socialRequested,
  socialSucceeded,
  type SocialRequest,
} from './socialSlice'
import {
  tokenFailed,
  tokenRequested,
  tokenSucceeded,
  type TokenRequest,
} from './tokensSlice'
import {
  createFailed,
  createRequested,
  createStarted,
  createSucceeded,
  listFailed,
  listRequested,
  listSucceeded,
} from './repositoriesSlice'

function errorMessage(err: unknown): string {
  return err instanceof ApiClientError ? err.message : 'Request failed'
}

function* listRepositories(): SagaIterator {
  try {
    const items: RepoResponse[] = yield call(repositoriesApi.list)
    yield put(listSucceeded(items))
  } catch (err) {
    yield put(listFailed(errorMessage(err)))
  }
}

function* createRepository(action: PayloadAction<CreateRepoParams>): SagaIterator {
  yield put(createStarted())
  try {
    const repo: RepoResponse = yield call(repositoriesApi.create, action.payload)
    yield put(createSucceeded(repo))
    // Refresh so the list is correct even if another tab changed it.
    yield call(listRepositories)
  } catch (err) {
    yield put(createFailed(errorMessage(err)))
  }
}

function* fetchBrowse(action: PayloadAction<BrowseRequest>): SagaIterator {
  const { part, key, owner, name, ref, path, sha } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'overview':
        data = yield call(repositoriesApi.overview, owner, name)
        break
      case 'branches':
        data = yield call(repositoriesApi.branches, owner, name)
        break
      case 'commits':
        data = yield call(repositoriesApi.commits, owner, name, ref)
        break
      case 'detail':
        data = yield call(repositoriesApi.commit, owner, name, sha ?? '')
        break
      case 'tree':
        data = yield call(repositoriesApi.tree, owner, name, ref, path)
        break
      case 'file':
        data = yield call(repositoriesApi.rawText, owner, name, ref, path ?? '')
        break
    }
    yield put(browseSucceeded({ part, key, data }))
  } catch (err) {
    yield put(browseFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchIssue(action: PayloadAction<IssueRequest>): SagaIterator {
  const { part, key, owner, name, number = 0, state, listState, params } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(issuesApi.list, owner, name, state)
        break
      case 'detail':
        data = yield call(issuesApi.show, owner, name, number)
        break
      case 'comments':
        data = yield call(issuesApi.comments, owner, name, number)
        break
      case 'labels':
        data = yield call(issuesApi.labels, owner, name)
        break
      case 'create':
        data = yield call(issuesApi.create, owner, name, params as CreateIssueParams)
        break
      case 'comment':
        data = yield call(
          issuesApi.comment,
          owner,
          name,
          number,
          params as AddCommentParams,
        )
        break
      case 'state':
        data = yield call(
          state === 'closed' ? issuesApi.close : issuesApi.reopen,
          owner,
          name,
          number,
        )
        break
      case 'createLabel':
        data = yield call(issuesApi.createLabel, owner, name, params as CreateLabelParams)
        break
    }
    yield put(issueSucceeded({ part, key, data }))

    // Mutations refresh the views that depend on them.
    if (part === 'create' || part === 'comment' || part === 'state' || part === 'createLabel') {
      const scope = `${owner}/${name}`
      yield put(
        issueRequested({
          part: 'list',
          key: `list:${scope}:${listState ?? ''}`,
          owner,
          name,
          state: listState,
        }),
      )
      if (number > 0) {
        yield put(
          issueRequested({
            part: 'detail',
            key: `detail:${scope}:${number}`,
            owner,
            name,
            number,
          }),
        )
        yield put(
          issueRequested({
            part: 'comments',
            key: `comments:${scope}:${number}`,
            owner,
            name,
            number,
          }),
        )
      }
    }
  } catch (err) {
    yield put(issueFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchPull(action: PayloadAction<PullRequestAction>): SagaIterator {
  const {
    part,
    key,
    owner,
    name,
    number = 0,
    state,
    listState,
    head,
    base,
    params,
  } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(pullsApi.list, owner, name, state)
        break
      case 'detail':
        data = yield call(pullsApi.show, owner, name, number)
        break
      case 'compare':
        data = yield call(pullsApi.compare, owner, name, head ?? '', base ?? '')
        break
      case 'comments':
        data = yield call(pullsApi.comments, owner, name, number)
        break
      case 'create':
        data = yield call(pullsApi.create, owner, name, params as CreatePullRequestParams)
        break
      case 'comment':
        data = yield call(
          pullsApi.comment,
          owner,
          name,
          number,
          params as AddReviewCommentParams,
        )
        break
      case 'state':
        data = yield call(
          state === 'closed' ? pullsApi.close : pullsApi.reopen,
          owner,
          name,
          number,
        )
        break
      case 'merge':
        data = yield call(pullsApi.merge, owner, name, number)
        break
    }
    yield put(pullSucceeded({ part, key, data }))

    if (part === 'create' || part === 'comment' || part === 'state' || part === 'merge') {
      const scope = `${owner}/${name}`
      yield put(
        pullRequested({
          part: 'list',
          key: `list:${scope}:${listState ?? ''}`,
          owner,
          name,
          state: listState,
        }),
      )
      if (number > 0) {
        yield put(
          pullRequested({
            part: 'detail',
            key: `detail:${scope}:${number}`,
            owner,
            name,
            number,
          }),
        )
        yield put(
          pullRequested({
            part: 'comments',
            key: `comments:${scope}:${number}`,
            owner,
            name,
            number,
          }),
        )
      }
    }
  } catch (err) {
    yield put(pullFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchCollaborator(action: PayloadAction<CollaboratorRequest>): SagaIterator {
  const { part, key, owner, name, username, role, userId } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(collaboratorsApi.list, owner, name)
        break
      case 'add':
        data = yield call(collaboratorsApi.add, owner, name, {
          username: username ?? '',
          role: role ?? 'read',
        })
        break
      case 'update':
        data = yield call(collaboratorsApi.update, owner, name, userId ?? 0, {
          role: role ?? 'read',
        })
        break
      case 'remove':
        yield call(collaboratorsApi.remove, owner, name, userId ?? 0)
        // The DELETE body is empty; hand the slice the id it needs to prune.
        data = { user_id: userId ?? 0 }
        break
    }
    yield put(collaboratorSucceeded({ part, key, data }))
  } catch (err) {
    yield put(collaboratorFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchSocial(action: PayloadAction<SocialRequest>): SagaIterator {
  const { part, key, owner, name } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'star':
        data = yield call(socialApi.star, owner, name)
        break
      case 'unstar':
        data = yield call(socialApi.unstar, owner, name)
        break
      case 'watch':
        data = yield call(socialApi.watch, owner, name)
        break
      case 'unwatch':
        data = yield call(socialApi.unwatch, owner, name)
        break
      case 'events':
        data = yield call(socialApi.events, owner, name)
        break
    }
    yield put(socialSucceeded({ part, key, data }))
    // A star/watch is itself an activity event, so refresh the feed.
    if (part !== 'events') {
      yield put(
        socialRequested({ part: 'events', key: `events:${owner}/${name}`, owner, name }),
      )
    }
  } catch (err) {
    yield put(socialFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchToken(action: PayloadAction<TokenRequest>): SagaIterator {
  const { part, key, params, id } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(tokensApi.list)
        break
      case 'create':
        data = yield call(tokensApi.create, params as CreateTokenParams)
        break
      case 'revoke':
        data = yield call(tokensApi.revoke, id ?? 0)
        break
    }
    yield put(tokenSucceeded({ part, key, data }))
    if (part !== 'list') {
      yield put(tokenRequested({ part: 'list', key: 'list:tokens' }))
    }
  } catch (err) {
    yield put(tokenFailed({ part, key, error: errorMessage(err) }))
  }
}

/** Root saga: all API side effects live here. */
export default function* rootSaga(): SagaIterator {
  yield all([
    takeLatest(listRequested.type, listRepositories),
    takeEvery(createRequested.type, createRepository),
    takeLatest(browseRequested.type, fetchBrowse),
    // takeEvery (not takeLatest): several issue requests run in parallel and
    // stale responses are dropped by the key stored in issuesSlice.
    takeEvery(issueRequested.type, fetchIssue),
    takeEvery(pullRequested.type, fetchPull),
    takeEvery(collaboratorRequested.type, fetchCollaborator),
    takeEvery(socialRequested.type, fetchSocial),
    takeEvery(tokenRequested.type, fetchToken),
  ])
}
