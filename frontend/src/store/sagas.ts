import type { PayloadAction } from '@reduxjs/toolkit'
import { all, call, put, takeEvery, takeLatest } from 'redux-saga/effects'
import type { SagaIterator } from 'redux-saga'
import { ApiClientError } from '../api/client'
import { repositoriesApi } from '../api/repositories'
import type { CreateRepoParams } from '../bindings/CreateRepoParams'
import type { RepoResponse } from '../bindings/RepoResponse'
import {
  browseFailed,
  browseRequested,
  browseSucceeded,
  type BrowseRequest,
} from './browseSlice'
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

/** Root saga: all API side effects for repository state live here. */
export default function* rootSaga(): SagaIterator {
  yield all([
    takeLatest(listRequested.type, listRepositories),
    takeEvery(createRequested.type, createRepository),
    takeLatest(browseRequested.type, fetchBrowse),
  ])
}
