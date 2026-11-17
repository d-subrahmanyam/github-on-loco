import type { PayloadAction } from '@reduxjs/toolkit'
import { all, call, put, takeEvery, takeLatest } from 'redux-saga/effects'
import type { SagaIterator } from 'redux-saga'
import { ApiClientError } from '../api/client'
import { issuesApi } from '../api/issues'
import { pullsApi } from '../api/pulls'
import { repositoriesApi } from '../api/repositories'
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
    takeEvery(tokenRequested.type, fetchToken),
  ])
}
