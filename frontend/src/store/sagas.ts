import type { PayloadAction } from '@reduxjs/toolkit'
import { all, call, put, takeEvery, takeLatest } from 'redux-saga/effects'
import type { SagaIterator } from 'redux-saga'
import { ApiClientError } from '../api/client'
import { collaboratorsApi } from '../api/collaborators'
import { forksApi } from '../api/forks'
import { issuesApi } from '../api/issues'
import { organizationsApi } from '../api/organizations'
import { protectionApi } from '../api/protection'
import { pullsApi } from '../api/pulls'
import { releasesApi } from '../api/releases'
import { repositoriesApi } from '../api/repositories'
import { socialApi } from '../api/social'
import { tokensApi } from '../api/tokens'
import type { AddCommentParams } from '../bindings/AddCommentParams'
import type { AddOrgMemberParams } from '../bindings/AddOrgMemberParams'
import type { AddReviewCommentParams } from '../bindings/AddReviewCommentParams'
import type { CreateIssueParams } from '../bindings/CreateIssueParams'
import type { CreateOrganizationParams } from '../bindings/CreateOrganizationParams'
import type { CreateTeamParams } from '../bindings/CreateTeamParams'
import type { GrantTeamRepoParams } from '../bindings/GrantTeamRepoParams'
import type { CreatePullRequestParams } from '../bindings/CreatePullRequestParams'
import type { CreateReleaseParams } from '../bindings/CreateReleaseParams'
import type { CreateLabelParams } from '../bindings/CreateLabelParams'
import type { CreateRepoParams } from '../bindings/CreateRepoParams'
import type { CreateTokenParams } from '../bindings/CreateTokenParams'
import type { RepoResponse } from '../bindings/RepoResponse'
import type { SetBranchProtectionParams } from '../bindings/SetBranchProtectionParams'
import type { SetReviewParams } from '../bindings/SetReviewParams'
import type { UpdateReleaseParams } from '../bindings/UpdateReleaseParams'
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
  forkFailed,
  forkRequested,
  forkSucceeded,
  type ForkRequest,
} from './forksSlice'
import {
  issueFailed,
  issueRequested,
  issueSucceeded,
  type IssueRequest,
} from './issuesSlice'
import {
  orgFailed,
  orgRequested,
  orgSucceeded,
  type OrgRequest,
} from './organizationsSlice'
import {
  protectionFailed,
  protectionRequested,
  protectionSucceeded,
  type ProtectionRequest,
} from './protectionSlice'
import {
  pullFailed,
  pullRequested,
  pullSucceeded,
  type PullRequest as PullRequestAction,
} from './pullsSlice'
import {
  releaseFailed,
  releaseRequested,
  releaseSucceeded,
  type ReleaseRequest,
} from './releasesSlice'
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

function* fetchFork(action: PayloadAction<ForkRequest>): SagaIterator {
  const { part, key, owner, name, forkName } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(forksApi.list, owner, name)
        break
      case 'create':
        data = yield call(forksApi.create, owner, name, {
          name: forkName || null,
        })
        break
    }
    yield put(forkSucceeded({ part, key, data }))
  } catch (err) {
    yield put(forkFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchRelease(action: PayloadAction<ReleaseRequest>): SagaIterator {
  const { part, key, owner, name, id = 0, tag = '', params } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(releasesApi.list, owner, name)
        break
      case 'show':
        data = yield call(releasesApi.show, owner, name, id)
        break
      case 'create':
        data = yield call(releasesApi.create, owner, name, params as CreateReleaseParams)
        break
      case 'update':
        data = yield call(releasesApi.update, owner, name, id, params as UpdateReleaseParams)
        break
      case 'remove':
        yield call(releasesApi.remove, owner, name, id)
        data = id
        break
      case 'tags':
        data = yield call(releasesApi.tags, owner, name)
        break
      case 'removeTag':
        yield call(releasesApi.removeTag, owner, name, tag)
        data = tag
        break
    }
    yield put(releaseSucceeded({ part, key, data }))

    if (part === 'create' || part === 'update' || part === 'remove' || part === 'removeTag') {
      yield put(
        releaseRequested({
          part: 'list',
          key: `releases:${owner}/${name}`,
          owner,
          name,
        }),
      )
      yield put(
        releaseRequested({
          part: 'tags',
          key: `tags:${owner}/${name}`,
          owner,
          name,
        }),
      )
    }
  } catch (err) {
    yield put(releaseFailed({ part, key, error: errorMessage(err) }))
  }
}

function* fetchOrg(action: PayloadAction<OrgRequest>): SagaIterator {
  const { part, key, handle = '', teamId = 0, userId = 0, repoId = 0, params } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(organizationsApi.list)
        break
      case 'show':
        data = yield call(organizationsApi.show, handle)
        break
      case 'create':
        data = yield call(
          organizationsApi.create,
          params as CreateOrganizationParams,
        )
        break
      case 'members':
        data = yield call(organizationsApi.members, handle)
        break
      case 'addMember':
        data = yield call(organizationsApi.addMember, handle, params as AddOrgMemberParams)
        break
      case 'removeMember':
        yield call(organizationsApi.removeMember, handle, userId)
        data = { user_id: userId }
        break
      case 'teams':
        data = yield call(organizationsApi.teams, handle)
        break
      case 'createTeam':
        data = yield call(organizationsApi.createTeam, handle, params as CreateTeamParams)
        break
      case 'teamMembers':
        data = {
          teamId,
          items: yield call(organizationsApi.teamMembers, handle, teamId),
        }
        break
      case 'addTeamMember': {
        const member = yield call(
          organizationsApi.addTeamMember,
          handle,
          teamId,
          params as AddOrgMemberParams,
        )
        data = { ...member, team_id: teamId }
        break
      }
      case 'removeTeamMember':
        yield call(organizationsApi.removeTeamMember, handle, teamId, userId)
        data = { teamId, userId }
        break
      case 'teamRepos':
        data = {
          teamId,
          items: yield call(organizationsApi.teamRepos, handle, teamId),
        }
        break
      case 'grantRepo':
        data = yield call(
          organizationsApi.grantRepo,
          handle,
          teamId,
          params as GrantTeamRepoParams,
        )
        break
      case 'revokeRepo':
        yield call(organizationsApi.revokeRepo, handle, teamId, repoId)
        data = { teamId, repoId }
        break
    }
    yield put(orgSucceeded({ part, key, data }))

    if (part === 'create') {
      yield put(orgRequested({ part: 'list', key: 'list:orgs' }))
    }
    if (part === 'addMember' || part === 'removeMember') {
      yield put(orgRequested({ part: 'members', key: `members:${handle}`, handle }))
    }
    if (part === 'createTeam') {
      yield put(orgRequested({ part: 'teams', key: `teams:${handle}`, handle }))
    }
    if (part === 'addTeamMember' || part === 'removeTeamMember') {
      yield put(
        orgRequested({
          part: 'teamMembers',
          key: `team-members:${handle}:${teamId}`,
          handle,
          teamId,
        }),
      )
      yield put(orgRequested({ part: 'teams', key: `teams:${handle}`, handle }))
    }
    if (part === 'grantRepo' || part === 'revokeRepo') {
      yield put(
        orgRequested({
          part: 'teamRepos',
          key: `team-repos:${handle}:${teamId}`,
          handle,
          teamId,
        }),
      )
    }
  } catch (err) {
    yield put(orgFailed({ part, key, error: errorMessage(err) }))
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

function* fetchProtection(action: PayloadAction<ProtectionRequest>): SagaIterator {
  const { part, key, owner, name, number = 0, branch = '', params } = action.payload
  try {
    let data: unknown
    switch (part) {
      case 'list':
        data = yield call(protectionApi.list, owner, name)
        break
      case 'set':
        data = yield call(
          protectionApi.set,
          owner,
          name,
          params as SetBranchProtectionParams,
        )
        break
      case 'remove':
        yield call(protectionApi.remove, owner, name, branch)
        // DELETE has no body; hand the slice the branch it needs to prune.
        data = { branch }
        break
      case 'reviews':
        data = yield call(protectionApi.reviews, owner, name, number)
        break
      case 'review':
        data = yield call(
          protectionApi.review,
          owner,
          name,
          number,
          params as SetReviewParams,
        )
        break
    }
    yield put(protectionSucceeded({ part, key, data }))

    const scope = `${owner}/${name}`
    if (part === 'set' || part === 'remove') {
      yield put(
        protectionRequested({ part: 'list', key: `list:${scope}`, owner, name }),
      )
    }
    if (part === 'review') {
      // Reviews are upserted, so reload the whole list and the PR's approval
      // count from the server rather than guessing locally.
      yield put(
        protectionRequested({
          part: 'reviews',
          key: `reviews:${scope}:${number}`,
          owner,
          name,
          number,
        }),
      )
      yield put(
        pullRequested({
          part: 'detail',
          key: `detail:${scope}:${number}`,
          owner,
          name,
          number,
        }),
      )
    }
  } catch (err) {
    yield put(protectionFailed({ part, key, error: errorMessage(err) }))
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
    takeEvery(forkRequested.type, fetchFork),
    takeEvery(releaseRequested.type, fetchRelease),
    takeEvery(orgRequested.type, fetchOrg),
    takeEvery(issueRequested.type, fetchIssue),
    takeEvery(pullRequested.type, fetchPull),
    takeEvery(collaboratorRequested.type, fetchCollaborator),
    takeEvery(protectionRequested.type, fetchProtection),
    takeEvery(socialRequested.type, fetchSocial),
    takeEvery(tokenRequested.type, fetchToken),
  ])
}
