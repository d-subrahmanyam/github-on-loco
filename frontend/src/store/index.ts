import { configureStore } from '@reduxjs/toolkit'
import createSagaMiddleware from 'redux-saga'
import browseReducer from './browseSlice'
import collaboratorsReducer from './collaboratorsSlice'
import forksReducer from './forksSlice'
import issuesReducer from './issuesSlice'
import organizationsReducer from './organizationsSlice'
import protectionReducer from './protectionSlice'
import pullsReducer from './pullsSlice'
import releasesReducer from './releasesSlice'
import searchReducer from './searchSlice'
import repositoriesReducer from './repositoriesSlice'
import rootSaga from './sagas'
import socialReducer from './socialSlice'
import tokensReducer from './tokensSlice'
import webhooksReducer from './webhooksSlice'

/** Build a fresh store with its own saga runtime. Tests use this for isolation. */
export function createAppStore() {
  const sagaMiddleware = createSagaMiddleware()
  const store = configureStore({
    reducer: {
      repositories: repositoriesReducer,
      browse: browseReducer,
      collaborators: collaboratorsReducer,
      forks: forksReducer,
      social: socialReducer,
      issues: issuesReducer,
      organizations: organizationsReducer,
      pulls: pullsReducer,
      releases: releasesReducer,
      search: searchReducer,
      protection: protectionReducer,
      tokens: tokensReducer,
      webhooks: webhooksReducer,
    },
    // Saga owns side effects; the default async thunk middleware is not needed.
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({ thunk: false }).concat(sagaMiddleware),
  })
  sagaMiddleware.run(rootSaga)
  return store
}

/** The single store used by the app. */
export const store = createAppStore()

export type RootState = ReturnType<typeof store.getState>
export type AppDispatch = typeof store.dispatch
